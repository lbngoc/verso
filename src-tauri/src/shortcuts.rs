//! Global shortcuts from settings, re-registered when settings change; conflicts are reported to
//! the UI so Settings can ask for another combination (FR-009, research P6).

use log::{info, warn};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::commands::{self, AppState};
use crate::{tray, window_manager};

#[derive(Debug, Clone, Copy)]
enum Action {
    Translate,
    Quick,
}

pub fn register_all(app: &AppHandle) -> Result<(), String> {
    let settings = app.state::<AppState>().settings.get();
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();

    let translate = register_one(app, &settings.shortcuts.translate, Action::Translate);
    let quick = register_one(app, &settings.shortcuts.quick, Action::Quick);

    let mut conflict = None;
    if let Err(error) = &translate {
        conflict = Some((settings.shortcuts.translate.clone(), error.clone()));
        warn!("{error}");
    }
    if let Err(error) = &quick {
        conflict.get_or_insert_with(|| (settings.shortcuts.quick.clone(), error.clone()));
        warn!("{error}");
    }
    if let Some((shortcut, reason)) = conflict {
        let _ = app.emit(
            "shortcut-conflict",
            serde_json::json!({ "shortcut": shortcut, "reason": reason }),
        );
        return Err(reason);
    }
    info!("global shortcuts registered");
    Ok(())
}

fn register_one(app: &AppHandle, value: &str, action: Action) -> Result<(), String> {
    let shortcut: Shortcut = value
        .parse()
        .map_err(|_| format!("'{value}' is not a valid shortcut"))?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                match action {
                    Action::Translate => trigger_translate(app),
                    Action::Quick => window_manager::toggle_quick(app),
                }
            }
        })
        .map_err(|error| format!("'{value}' is already taken: {error}"))
}

pub fn trigger_translate(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tray::set_busy(&handle, true);
        let state = handle.state::<AppState>();
        // Success or failure, the mark goes back to idle (spec 008 FR-010).
        let _ = commands::translate_selection(handle.clone(), state).await;
        tray::set_busy(&handle, false);
    });
}
