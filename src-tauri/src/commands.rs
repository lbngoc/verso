//! Tauri IPC commands and the events the UI listens to (ui-ipc-contract.md).

use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::core::capture::outcome_to_popup_state;
use crate::core::engine::{Availability, EngineInfo, Origin, TranslationResult};
use crate::core::errors::EngineError;
use crate::core::langdetect;
use crate::core::languages::{self, Language};
use crate::core::modules::{self, ModuleInfo};
use crate::core::pipeline::Pipeline;
use crate::core::registry::Registry;
use crate::core::settings::{SettingsProfile, SettingsStore};
use crate::engines::http_bing::HttpBingEngine;
use crate::engines::http_google::HttpGoogleEngine;
use crate::platform::{self, Appearance, PermissionKind, PermissionStatus};
use crate::report::{self, ReportContext};
use crate::window_manager;

/// Everything the commands need; built once at startup and managed by Tauri.
pub struct AppState {
    pub settings: Arc<SettingsStore>,
    pub registry: Arc<Registry>,
    pub pipeline: Arc<Pipeline>,
}

impl AppState {
    pub fn new(settings: Arc<SettingsStore>) -> Result<Self, String> {
        let google = HttpGoogleEngine::new().map_err(|error| format!("Google engine: {error}"))?;
        let bing = HttpBingEngine::new().map_err(|error| format!("Bing engine: {error}"))?;
        let registry = Arc::new(Registry::new(
            vec![Arc::new(google), Arc::new(bing)],
            settings.clone(),
        ));
        let pipeline = Arc::new(Pipeline::new(registry.clone(), settings.clone()));
        Ok(Self {
            settings,
            registry,
            pipeline,
        })
    }
}

#[derive(Serialize, Clone)]
struct TranslationStarted {
    origin: Origin,
    truncated: bool,
    original_length: Option<usize>,
}

#[tauri::command]
pub async fn translate(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    source: String,
    target: String,
    origin: Option<Origin>,
) -> Result<TranslationResult, EngineError> {
    let origin = origin.unwrap_or(Origin::Typed);
    let total = text.chars().count();
    let truncated = total > crate::core::pipeline::MAX_INPUT_CHARS;
    let _ = app.emit(
        "translation-started",
        TranslationStarted {
            origin,
            truncated,
            original_length: if truncated { Some(total) } else { None },
        },
    );
    let result = state
        .pipeline
        .translate(&text, &source, &target, origin)
        .await;
    match &result {
        Ok(value) => {
            let _ = app.emit("translation-finished", value);
        }
        Err(error) => {
            let _ = app.emit("translation-finished", json!({ "error": error }));
        }
    }
    result
}

/// Shows the popup, captures the selection and translates it (US1 main flow).
#[tauri::command]
pub async fn translate_selection(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<TranslationResult>, EngineError> {
    let started = Instant::now();
    let expanded = state.settings.get().popup_expanded;
    let _ = window_manager::show_popup(&app, expanded);

    let outcome = tokio::task::spawn_blocking(|| platform::services().capture_selection())
        .await
        .map_err(|error| EngineError::internal(format!("capture task failed: {error}")))?;

    log::info!(
        "capture: method={:?} failure={:?} clipboard={:?} app={:?}",
        outcome.method,
        outcome.failure,
        outcome.clipboard,
        outcome.frontmost_app
    );
    let mut reported = outcome.clone();
    reported.text = None;
    let _ = app.emit("capture-finished", &reported);
    let plan = outcome_to_popup_state(&outcome);
    let _ = app.emit("popup-state", &plan.popup);

    if plan.open_onboarding {
        window_manager::hide_popup(&app);
        window_manager::show_permission_onboarding(&app);
        return Ok(None);
    }

    if plan.open_quick || matches!(plan.popup, crate::core::capture::PopupState::Hidden) {
        window_manager::hide_popup(&app);
        let _ = window_manager::show_quick(&app);
        let _ = app.emit("popup-state", crate::core::capture::PopupState::Hidden);
        return Ok(None);
    }

    let Some(text) = outcome.text.clone() else {
        return Ok(None);
    };

    let settings = state.settings.get();
    let total = text.chars().count();
    let truncated = total > crate::core::pipeline::MAX_INPUT_CHARS;
    let _ = app.emit(
        "translation-started",
        TranslationStarted {
            origin: Origin::Selection,
            truncated,
            original_length: if truncated { Some(total) } else { None },
        },
    );
    let result = state
        .pipeline
        .translate(
            &text,
            &settings.source_lang,
            &settings.target_lang,
            Origin::Selection,
        )
        .await;

    match &result {
        Ok(value) => {
            let _ = app.emit("translation-finished", value);
            let _ = app.emit("popup-state", crate::core::capture::PopupState::Result);
        }
        Err(error) => {
            let _ = app.emit("translation-finished", json!({ "error": error }));
            let _ = app.emit(
                "popup-state",
                crate::core::capture::PopupState::Error {
                    error: error.clone(),
                },
            );
        }
    }

    crate::core::logging::debug_duration("translate_selection", started);
    Ok(result.ok())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> SettingsProfile {
    state.settings.get()
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: Value,
) -> Result<SettingsProfile, String> {
    let profile = state
        .settings
        .update(&patch)
        .map_err(|error| error.to_string())?;
    let _ = app.emit("settings-changed", &profile);
    crate::appearance::refresh_with(&app);
    let _ = crate::shortcuts::register_all(&app);
    crate::autostart::apply(&app, profile.launch_at_login);
    Ok(profile)
}

#[tauri::command]
pub fn list_engines(state: State<'_, AppState>) -> Vec<EngineInfo> {
    state.registry.engine_infos()
}

#[tauri::command]
pub fn set_engine_order(
    state: State<'_, AppState>,
    order: Vec<String>,
) -> Result<Vec<EngineInfo>, String> {
    state
        .settings
        .update(&json!({ "engine_order": order }))
        .map_err(|error| error.to_string())?;
    Ok(state.registry.engine_infos())
}

#[tauri::command]
pub fn speak(text: String, lang: String) -> Result<(), String> {
    platform::services().speak(&text, &lang)
}

#[tauri::command]
pub fn check_permissions(kind: PermissionKind) -> PermissionStatus {
    platform::services().permission_status(kind)
}

#[tauri::command]
pub fn request_permission(kind: PermissionKind) -> PermissionStatus {
    platform::services().request_permission(kind);
    platform::services().permission_status(kind)
}

#[tauri::command]
pub fn detect_language(text: String) -> Option<String> {
    langdetect::detect_language(&text)
}

#[tauri::command]
pub fn list_modules() -> Vec<ModuleInfo> {
    modules::list_modules()
}

#[tauri::command]
pub fn list_languages() -> Vec<Language> {
    languages::all().to_vec()
}

#[tauri::command]
pub fn get_appearance(app: AppHandle) -> Appearance {
    crate::appearance::current(&app)
}

#[tauri::command]
pub fn get_capture_outcome(plan: Value) -> Value {
    // Placeholder for the UI: the real outcome arrives through `capture-finished`.
    plan
}

#[tauri::command]
pub fn hide_popup(app: AppHandle) {
    window_manager::hide_popup(&app);
}

#[tauri::command]
pub fn show_quick(app: AppHandle) {
    let _ = window_manager::show_quick(&app);
}

#[tauri::command]
pub fn hide_quick(app: AppHandle) {
    window_manager::hide_quick(&app);
}

// Async on purpose: building a webview window inside a synchronous command deadlocks WebView2 on
// Windows (the frame shows, the page never loads).
#[tauri::command]
pub async fn open_settings(app: AppHandle) {
    window_manager::show_settings(&app);
}

// Async on purpose: building a webview window inside a synchronous command deadlocks WebView2 on
// Windows (the frame shows, the page never loads).
#[tauri::command]
pub async fn open_onboarding(app: AppHandle) {
    window_manager::show_onboarding(&app);
}

#[tauri::command]
pub fn onboarding_step() -> u8 {
    window_manager::onboarding_step()
}

#[tauri::command]
pub fn settings_section() -> Option<String> {
    window_manager::take_settings_section()
}

#[tauri::command]
pub fn set_popup_expanded(
    app: AppHandle,
    state: State<'_, AppState>,
    expanded: bool,
) -> Result<SettingsProfile, String> {
    let profile = state
        .settings
        .update(&json!({ "popup_expanded": expanded }))
        .map_err(|error| error.to_string())?;
    let _ = app.emit("settings-changed", &profile);
    window_manager::resize_popup(&app, expanded);
    Ok(profile)
}

/// First-run engine choice (FR-016): stores the disabled engines and the order, then marks the step done.
#[tauri::command]
pub fn complete_engine_choice(
    app: AppHandle,
    state: State<'_, AppState>,
    engines_disabled: Vec<String>,
    engine_order: Vec<String>,
) -> Result<SettingsProfile, String> {
    let stamp = format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    );
    let profile = state
        .settings
        .update(&json!({
            "engines_disabled": engines_disabled,
            "engine_order": engine_order,
            "first_run_engine_choice_at": stamp,
        }))
        .map_err(|error| error.to_string())?;
    let _ = app.emit("settings-changed", &profile);
    Ok(profile)
}

#[tauri::command]
pub fn voice_exists(lang: String) -> bool {
    platform::services().voice_exists(&lang)
}

#[tauri::command]
pub fn engine_availability(state: State<'_, AppState>) -> Vec<(String, Availability)> {
    state
        .registry
        .engine_infos()
        .into_iter()
        .map(|info| (info.id, info.availability))
        .collect()
}

/// Everything a problem report may carry; built from the app, never from the webview.
fn report_context(app: &AppHandle, profile: &SettingsProfile) -> ReportContext {
    let appearance = platform::services().appearance();
    ReportContext {
        version: app.package_info().version.to_string(),
        os: match appearance.os {
            platform::Os::Macos => "macos".into(),
            platform::Os::Windows => "windows".into(),
        },
        os_version: appearance.os_version,
        arch: std::env::consts::ARCH.into(),
        engines: profile.engine_order.clone(),
    }
}

fn log_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_log_dir()
        .ok()
        .map(|directory| directory.join("verso.log"))
}

fn home_directory() -> Option<String> {
    dirs::home_dir().map(|home| home.to_string_lossy().to_string())
}

fn redacted_log(app: &AppHandle) -> String {
    let Some(path) = log_path(app) else {
        return String::new();
    };
    report::read_log_tail(&path, report::MAX_DIAGNOSTICS_BYTES)
}

/// Opens the prefilled bug form in the default browser (FR-018). Nothing is sent by the app.
#[tauri::command]
pub fn report_problem(app: AppHandle) -> Result<String, String> {
    use tauri_plugin_opener::OpenerExt;

    let profile = app.state::<AppState>().settings.get();
    let context = report_context(&app, &profile);
    let log = redacted_log(&app);
    let home = home_directory();
    let url = report::build_issue_url(&context, &log, home.as_deref());
    app.opener()
        .open_url(url.clone(), None::<&str>)
        .map_err(|error| format!("could not open the browser: {error}"))?;
    log::info!(
        "problem report opened ({url_width} chars)",
        url_width = url.len()
    );
    Ok(url)
}

/// Copies the diagnostics text for the user to review and paste into the issue.
#[tauri::command]
pub fn copy_diagnostics(app: AppHandle) -> Result<String, String> {
    let profile = app.state::<AppState>().settings.get();
    let context = report_context(&app, &profile);
    let log = redacted_log(&app);
    let home = home_directory();
    let text = report::diagnostics(&context, &profile, &log, home.as_deref());
    platform::services().write_clipboard_text(&text)?;
    Ok(text)
}
