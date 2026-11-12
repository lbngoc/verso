pub mod appearance;
pub mod autostart;
pub mod commands;
pub mod core;
pub mod engines;
pub mod platform;
pub mod report;
pub mod shortcuts;
pub mod tray;
pub mod updater;
pub mod window_manager;

use std::sync::Arc;

use tauri::{Emitter, Manager};

use crate::commands::AppState;
use crate::core::settings::{SettingsStore, SETTINGS_FILE};

fn settings_path() -> std::path::PathBuf {
    // Same location as Tauri's app_data_dir for identifier app.versotranslate, but resolvable before
    // the app exists: window webviews can call commands as soon as they load, which may be before
    // `setup` runs (the Windows startup race fixed in 2026-11-08).
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("app.versotranslate")
        .join(SETTINGS_FILE)
}

fn self_test_iterations() -> Option<u32> {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--self-test" {
            return arguments.next().and_then(|value| value.parse().ok());
        }
        if let Some(value) = argument.strip_prefix("--self-test=") {
            return value.parse().ok();
        }
    }
    None
}

pub fn run() {
    // Installed before the builder: Tauri creates the configured popup/quick windows before
    // `setup` runs, and their webviews may call commands immediately (Windows startup race).
    platform::init();

    let settings = Arc::new(SettingsStore::open(settings_path()));
    let state = match AppState::new(settings.clone()) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("Verso could not start its engines: {error}");
            return;
        }
    };

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            window_manager::show_settings(app);
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(core::logging::plugin())
        .setup(|app| {
            let handle = app.handle().clone();

            log::info!(
                "accessibility permission at startup: {:?}",
                platform::services()
                    .permission_status(crate::platform::PermissionKind::Accessibility)
            );

            // Engine fallback events reach the UI (FR-008).
            {
                let emitter = handle.clone();
                let state = handle.state::<AppState>();
                state
                    .registry
                    .set_fallback_sink(Arc::new(move |from, to, error| {
                        let _ = emitter.emit(
                            "engine-fallback",
                            serde_json::json!({ "from": from, "to": to, "reason": error }),
                        );
                    }));
            }

            #[cfg(target_os = "macos")]
            {
                for label in ["popup", "quick"] {
                    if let Some(window) = handle.get_webview_window(label) {
                        platform::macos::panel::prepare(&window);
                    }
                }
            }
            #[cfg(windows)]
            {
                for label in ["popup", "quick"] {
                    if let Some(window) = handle.get_webview_window(label) {
                        platform::windows::window_style::prepare(&window);
                    }
                }
            }

            for label in ["popup", "quick"] {
                if let Some(window) = handle.get_webview_window(label) {
                    let handle = handle.clone();
                    window.on_window_event(move |event| {
                        if let tauri::WindowEvent::Focused(false) = event {
                            window_manager::hide_popup(&handle);
                            window_manager::hide_quick(&handle);
                        }
                    });
                }
            }

            window_manager::prewarm(&handle);
            appearance::init(&handle);
            appearance::watch(handle.clone());
            appearance::watch_permission(handle.clone());

            if let Err(error) = shortcuts::register_all(&handle) {
                log::warn!("shortcut registration: {error}");
            }
            let profile = state_settings(&handle);
            autostart::apply(&handle, profile.launch_at_login);
            tray::create(&handle)?;

            // T059: after an update macOS may keep the old Accessibility entry switched on while the new
            // build is not trusted. A version change with the grant missing opens the guided re-grant
            // step at once, rather than waiting for a failed capture.
            #[cfg(target_os = "macos")]
            {
                use crate::core::settings::needs_permission_regrant;
                use crate::platform::{PermissionKind, PermissionStatus};

                let granted = platform::services().permission_status(PermissionKind::Accessibility)
                    == PermissionStatus::Granted;
                if needs_permission_regrant(
                    profile.first_run_engine_choice_at.is_some(),
                    &profile.last_run_version,
                    env!("CARGO_PKG_VERSION"),
                    granted,
                ) {
                    log::info!(
                        "the code identity changed ({} -> {}) and Accessibility is missing; showing the re-grant step",
                        profile.last_run_version,
                        env!("CARGO_PKG_VERSION")
                    );
                    let handle = handle.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        window_manager::show_permission_onboarding(&handle);
                    });
                }
            }

            // Remember what ran now, so the next start can tell an update happened.
            if profile.last_run_version != env!("CARGO_PKG_VERSION") {
                let _ = handle.state::<AppState>().settings.update(&serde_json::json!({
                    "last_run_version": env!("CARGO_PKG_VERSION"),
                }));
            }

            if profile.first_run_engine_choice_at.is_none() {
                let handle = handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(350));
                    window_manager::show_onboarding(&handle);
                });
            }

            if let Some(iterations) = self_test_iterations() {
                let handle = handle.clone();
                std::thread::spawn(move || window_manager::self_test(&handle, iterations));
            }

            // Self-update schedule: first check after 30 s, then daily; silent until onboarding ends.
            updater::start(handle.clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::translate,
            commands::translate_selection,
            commands::get_settings,
            commands::update_settings,
            commands::list_engines,
            commands::set_engine_order,
            commands::speak,
            commands::check_permissions,
            commands::request_permission,
            commands::detect_language,
            commands::list_modules,
            commands::list_languages,
            commands::get_appearance,
            commands::hide_popup,
            commands::show_quick,
            commands::hide_quick,
            commands::open_settings,
            commands::open_onboarding,
            commands::onboarding_step,
            commands::settings_section,
            commands::set_popup_expanded,
            commands::complete_engine_choice,
            commands::voice_exists,
            commands::engine_availability,
            commands::report_problem,
            commands::copy_diagnostics,
            updater::check_for_update,
            updater::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Verso");
}

fn state_settings(app: &tauri::AppHandle) -> crate::core::settings::SettingsProfile {
    app.state::<AppState>().settings.get()
}
