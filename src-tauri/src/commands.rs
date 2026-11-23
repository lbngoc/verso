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
    // A second shortcut while the first is still working replaces the content: the older request
    // must not paint its result over the newer one (spec 009, T017).
    static REQUEST_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let request = REQUEST_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    let _ = window_manager::show_popup(&app);

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
        window_manager::show_permission_onboarding(
            &app,
            PermissionKind::Accessibility,
            platform::PermissionReason::Missing,
        );
        return Ok(None);
    }

    if plan.open_quick || matches!(plan.popup, crate::core::capture::PopupState::Hidden) {
        window_manager::hide_popup(&app);
        let _ = window_manager::show_quick(&app, window_manager::PanelAnchor::Shortcut);
        let _ = app.emit("popup-state", crate::core::capture::PopupState::Hidden);
        emit_quick_prefill(&app, state.settings.get().quick_paste_clipboard);
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

    // A newer request replaced this one while it was translating: drop the late result.
    if REQUEST_SEQUENCE.load(std::sync::atomic::Ordering::SeqCst) != request {
        log::info!("translate_selection: request {request} superseded; its result is dropped");
        return Ok(result.ok());
    }

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
    // Switching the module on is refused where it cannot run (spec 004 FR-012): the value stays off.
    let enabling = patch
        .get("ocr")
        .and_then(|ocr| ocr.get("enabled"))
        .and_then(Value::as_bool)
        == Some(true);
    if enabling && !platform::services().supports("ocr") {
        return Err("screen translation is not available on this system".into());
    }
    let was_enabled = state.settings.get().ocr.enabled;
    let profile = state
        .settings
        .update(&patch)
        .map_err(|error| error.to_string())?;
    let _ = app.emit("settings-changed", &profile);
    crate::appearance::refresh_with(&app);
    // Re-registers every shortcut: the screen one only while the module is on (FR-010, FR-013).
    let _ = crate::shortcuts::register_all(&app);
    crate::autostart::apply(&app, profile.launch_at_login);
    if profile.ocr.enabled != was_enabled {
        if !profile.ocr.enabled {
            // Releasing the module also closes a picker that is still open, and the warm overlay.
            crate::core::ocr::cancel(&app);
            window_manager::release_warm_overlay(&app);
        }
        let _ = app.emit("ocr-changed", json!({ "enabled": profile.ocr.enabled }));
    }
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

/// Why the permission step for `kind` is open (spec 010 contracts/permission-step.md).
#[tauri::command]
pub fn permission_reason(kind: PermissionKind) -> platform::PermissionReason {
    window_manager::permission_reason(kind)
}

/// "Reset and grant again": clears Verso's own entry and asks again; refused while the grant is
/// effective (spec 010 R2).
#[tauri::command]
pub async fn reset_permission(kind: PermissionKind) -> platform::ResetOutcome {
    tokio::task::spawn_blocking(move || platform::services().reset_permission(kind))
        .await
        .unwrap_or(platform::ResetOutcome {
            ok: false,
            status: platform::services().permission_status(kind),
        })
}

/// Whether screen translate can run here, and with which recognition languages (spec 004).
#[tauri::command]
pub fn ocr_availability() -> platform::OcrAvailability {
    static CACHE: std::sync::OnceLock<platform::OcrAvailability> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| platform::services().ocr_availability())
        .clone()
}

/// Same path as the shortcut and the tray item.
#[tauri::command]
pub async fn start_screen_translate(app: AppHandle) -> &'static str {
    match crate::core::ocr::start(&app) {
        crate::core::ocr::StartResult::Opened => "opened",
        crate::core::ocr::StartResult::Ignored => "ignored",
        crate::core::ocr::StartResult::Permission => "permission",
    }
}

/// The overlay page has painted: the backend shows that window.
#[tauri::command]
pub fn overlay_ready(app: AppHandle, monitor: u32) {
    window_manager::overlay_page_ready(&app, monitor);
}

/// The drag ended: capture, read, translate, show the popup next to the region.
#[tauri::command]
pub async fn finish_region(app: AppHandle, monitor: u32, rect: platform::Rect) {
    crate::core::ocr::finish(app, monitor, rect).await;
}

#[tauri::command]
pub fn cancel_region(app: AppHandle) {
    crate::core::ocr::cancel(&app);
}

/// The kind the permission step was opened for (Accessibility unless Screen Recording asked).
#[tauri::command]
pub fn permission_step_kind() -> PermissionKind {
    window_manager::permission_step_kind()
}

/// Relaunches Verso (macOS may apply a new Screen Recording grant only after a restart).
#[tauri::command]
pub fn restart_app(app: AppHandle) {
    log::info!("restart requested from the permission step");
    app.restart();
}

/// Version and build date for the About section (spec 009, contracts/settings-window.md).
#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub build_date: String,
}

/// `YYYY-MM-DD` from Unix seconds, UTC (no date crate for one line).
fn build_date(seconds: u64) -> String {
    let z = (seconds / 86_400) as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 {
        yoe + era * 400 + 1
    } else {
        yoe + era * 400
    };
    format!("{year:04}-{month:02}-{day:02}")
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    let seconds: u64 = env!("VERSO_BUILD_DATE").parse().unwrap_or(0);
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_date: build_date(seconds),
    }
}

/// Opens a link from the update notes in the default browser (http/https only, spec 009 T047).
#[tauri::command]
pub async fn open_update_window(app: AppHandle, mode: String) {
    window_manager::open_update_window(&app, if mode == "offer" { "offer" } else { "check" });
}

#[tauri::command]
pub fn hide_update_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("update") {
        let _ = window.close();
    }
}

/// The update page reports the height its content needs.
#[tauri::command]
pub fn resize_update_window(app: AppHandle, height: f64) {
    window_manager::resize_update_window(&app, height);
}

#[tauri::command]
pub fn update_mode() -> String {
    window_manager::take_update_mode()
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let parsed = url::Url::parse(&url).map_err(|error| format!("invalid url: {error}"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => return Err(format!("unsupported scheme: {other}")),
    }
    app.opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|error| error.to_string())
}

/// Opens one of the About pages by name; the URLs live in `links.rs`, not in the webview.
#[tauri::command]
pub fn open_public_page(app: AppHandle, page: String) -> Result<(), String> {
    let url = crate::links::page_url(&page).ok_or_else(|| format!("unknown page: {page}"))?;
    open_url(app, url.to_string())
}

#[tauri::command]
pub fn detect_language(text: String) -> Option<String> {
    langdetect::detect_language(&text)
}

/// Clipboard prefill (FR-011): only when the user opted in, and the read is guarded before it
/// happens (R9). Shared by the no-selection path and the tray click.
fn emit_quick_prefill(app: &AppHandle, enabled: bool) {
    let prefill = prefill_text(enabled, || platform::services().read_clipboard_text());
    if let Some(text) = prefill {
        window_manager::send_quick_prefill(app, text);
    }
}

/// Tray left click: toggles the quick panel, prefilled from the clipboard when the option is on
/// (FR-034). A click that arrives right after the panel hid (its own blur) just closes it.
pub fn open_quick_from_tray(app: &AppHandle, icon: crate::platform::Rect) {
    window_manager::remember_tray_icon(icon);
    if window_manager::quick_just_hidden() {
        log::info!("tray click: panel just hidden by this click; staying closed");
        return;
    }
    if window_manager::quick_is_open(app) {
        log::info!("tray click: panel open; closing it");
        window_manager::hide_quick(app);
        return;
    }
    log::info!("tray click: opening the panel");
    window_manager::hide_popup(app);
    if window_manager::show_quick(app, window_manager::PanelAnchor::TrayIcon(icon)).is_err() {
        return;
    }
    let enabled = app.state::<AppState>().settings.get().quick_paste_clipboard;
    emit_quick_prefill(app, enabled);
}

/// Panel prefill helper (FR-011): reads the clipboard only when the option is on.
///
/// Kept pure so the "never reads when disabled" rule can be asserted in a test.
pub fn prefill_text<F>(enabled: bool, read: F) -> Option<String>
where
    F: FnOnce() -> Option<String>,
{
    if !enabled {
        return None;
    }
    let text = read()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(
        trimmed
            .chars()
            .take(crate::core::pipeline::MAX_INPUT_CHARS)
            .collect(),
    )
}

/// Speaks the selection in its detected language; a second press stops (R8, FR-009).
pub async fn speak_selection(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if platform::services().is_speaking() {
        platform::services().stop_speaking();
        return Ok(());
    }
    let outcome = tokio::task::spawn_blocking(|| platform::services().capture_selection())
        .await
        .map_err(|error| format!("capture task failed: {error}"))?;
    if outcome.failure == Some(crate::platform::CaptureFailure::SecureField) {
        // Nothing read, nothing spoken.
        return Ok(());
    }
    let text = outcome
        .text
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    let Some(text) = text else {
        // The pill says "Nothing selected" for 1.5 s, then the popup hides itself.
        let _ = window_manager::show_popup(&app);
        let _ = app.emit(
            "popup-state",
            crate::core::capture::PopupState::Status {
                kind: crate::core::capture::StatusKind::NothingSelected,
            },
        );
        let handle = app.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            window_manager::hide_popup(&handle);
        });
        return Ok(());
    };
    let settings = state.settings.get();
    let lang = langdetect::detect_language(&text)
        .filter(|lang| lang != "und")
        .unwrap_or_else(|| {
            if settings.source_lang == "auto" {
                settings.target_lang.clone()
            } else {
                settings.source_lang.clone()
            }
        });
    log::info!(
        "speak selection: method={:?} chars={} lang={lang}",
        outcome.method,
        text.chars().count()
    );
    platform::services().speak(&text, &lang)
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

// Async on purpose: the first open builds the quick webview, and building one inside a synchronous
// command deadlocks WebView2 on Windows.
#[tauri::command]
pub async fn show_quick(app: AppHandle) {
    let _ = window_manager::show_quick(&app, window_manager::PanelAnchor::Shortcut);
}

/// The quick page mounted its listeners: a queued show or prefill can go now (T122).
#[tauri::command]
pub fn quick_ready(app: AppHandle) {
    window_manager::quick_page_ready(&app);
}

#[tauri::command]
pub fn hide_quick(app: AppHandle) {
    window_manager::hide_quick(&app);
}

/// The panel rendered the "Reading the text…" state; the show may go on (spec 004 FR-021).
#[tauri::command]
pub fn quick_screen_prepared() {
    window_manager::quick_screen_prepared();
}

/// The popup rendered the reset "Translating" state; the show may go on (FR-022).
#[tauri::command]
pub fn popup_prepared() {
    window_manager::popup_prepared();
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
pub fn set_popup_size(app: AppHandle, width: f64, height: f64) {
    window_manager::resize_popup_to(&app, width, height);
}

/// Global shortcuts are suspended while the recorder listens (R6).
#[tauri::command]
pub fn suspend_shortcuts(app: AppHandle, suspended: bool) {
    if suspended {
        crate::shortcuts::suspend(&app);
    } else {
        let _ = crate::shortcuts::resume(&app);
    }
}

#[tauri::command]
pub fn set_shortcut(
    app: AppHandle,
    action: String,
    accel: String,
) -> Result<SettingsProfile, crate::shortcuts::BindingError> {
    crate::shortcuts::set_binding(&app, &action, &accel)?;
    Ok(app.state::<AppState>().settings.get())
}

#[tauri::command]
pub fn reset_shortcut(
    app: AppHandle,
    action: String,
) -> Result<SettingsProfile, crate::shortcuts::BindingError> {
    crate::shortcuts::reset_binding(&app, &action)?;
    Ok(app.state::<AppState>().settings.get())
}

#[tauri::command]
pub fn set_popup_pinned(pinned: bool) {
    window_manager::POPUP_PINNED.store(pinned, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
pub fn focus_popup(app: AppHandle) {
    if let Some(window) = window_manager::popup_window(&app) {
        let visible = window.is_visible().unwrap_or(false);
        if !window_manager::popup_may_take_focus(visible, window_manager::quick_is_open(&app)) {
            return;
        }
        platform::services().focus_popup(&window);
    }
}

#[tauri::command]
pub fn stop_speaking() {
    platform::services().stop_speaking();
}

/// The text size step the popup and the panel share (FR-005); the page debounces the writes.
#[tauri::command]
pub fn set_text_size_step(
    app: AppHandle,
    state: State<'_, AppState>,
    step: i8,
) -> Result<SettingsProfile, String> {
    let step = step.clamp(-2, 4);
    let profile = state
        .settings
        .update(&json!({ "text_size_step": step }))
        .map_err(|error| error.to_string())?;
    let _ = app.emit("settings-changed", &profile);
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

#[cfg(test)]
mod tests {
    use super::prefill_text;
    use crate::core::pipeline::MAX_INPUT_CHARS;

    #[test]
    fn prefill_never_reads_the_clipboard_when_disabled() {
        let called = std::cell::Cell::new(false);
        let text = prefill_text(false, || {
            called.set(true);
            Some("bonjour".into())
        });
        assert_eq!(text, None);
        assert!(
            !called.get(),
            "the clipboard must not be read when the option is off"
        );
    }

    #[test]
    fn prefill_trims_and_caps_the_text() {
        let long = "x".repeat(MAX_INPUT_CHARS + 500);
        let text = prefill_text(true, || Some(format!("  {long}  "))).unwrap();
        assert_eq!(text.chars().count(), MAX_INPUT_CHARS);
        assert_eq!(prefill_text(true, || Some("   ".into())), None);
        assert_eq!(prefill_text(true, || None), None);
        assert_eq!(
            prefill_text(true, || Some("  hello  ".into())),
            Some("hello".into())
        );
    }
}
