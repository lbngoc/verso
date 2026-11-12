//! Launch at login via tauri-plugin-autostart (FR-009/FR-010).

use log::warn;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub fn apply(app: &AppHandle, enabled: bool) {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(error) = result {
        warn!("could not update launch-at-login: {error}");
    }
}

pub fn is_enabled(app: &AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}
