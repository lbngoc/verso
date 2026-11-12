//! App-level appearance state: combines the OS material with the user's theme override, applies it
//! to both transient windows and emits `appearance-changed` (research P9).

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use log::debug;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::AppState;
use crate::core::settings::Theme;
use crate::platform::{self, Appearance, Material, UiTheme};
use crate::window_manager;

static CURRENT: OnceLock<Mutex<Appearance>> = OnceLock::new();

fn slot() -> &'static Mutex<Appearance> {
    CURRENT.get_or_init(|| Mutex::new(platform::services().appearance()))
}

fn with_theme_override(mut appearance: Appearance, theme: Theme) -> Appearance {
    appearance.theme = match theme {
        Theme::System => appearance.theme,
        Theme::Light => UiTheme::Light,
        Theme::Dark => UiTheme::Dark,
    };
    if appearance.high_contrast || appearance.reduce_transparency {
        appearance.material = Material::Opaque;
    }
    appearance
}

pub fn init(app: &AppHandle) -> Appearance {
    let theme = app.state::<AppState>().settings.get().theme;
    let appearance = with_theme_override(platform::services().appearance(), theme);
    *slot().lock().unwrap() = appearance.clone();
    apply_to_windows(app, &appearance);
    appearance
}

pub fn current(app: &AppHandle) -> Appearance {
    let theme = app.state::<AppState>().settings.get().theme;
    let base = slot()
        .lock()
        .map(|value| value.clone())
        .unwrap_or_else(|_| platform::services().appearance());
    with_theme_override(base, theme)
}

pub fn apply_to_windows(app: &AppHandle, appearance: &Appearance) {
    // AppKit/DWM material calls belong on the main thread; commands arrive on worker threads.
    let handle = app.clone();
    let appearance = appearance.clone();
    let _ = app.run_on_main_thread(move || {
        for label in ["popup", "quick"] {
            if let Some(window) = handle.get_webview_window(label) {
                platform::services().apply_window_material(&window, &appearance);
            }
        }
    });
}

/// Recomputes after a settings change (theme override) and re-applies the native layer.
pub fn refresh_with(app: &AppHandle) {
    let appearance = with_theme_override(
        platform::services().appearance(),
        app.state::<AppState>().settings.get().theme,
    );
    let changed = {
        let mut guard = slot().lock().unwrap();
        let changed = *guard != appearance;
        *guard = appearance.clone();
        changed
    };
    if changed {
        apply_to_windows(app, &appearance);
        let _ = app.emit("appearance-changed", &appearance);
        debug!("appearance changed: {:?}", appearance);
    }
}

/// Polls the OS for appearance changes (Reduce Transparency, dark mode, contrast, accent).
pub fn watch(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(2));
        refresh_with(&app);
        crate::tray::refresh_theme(&app);
    });
}

pub fn watch_permission(app: AppHandle) {
    use crate::platform::{PermissionKind, PermissionStatus};
    let mut last = platform::services().permission_status(PermissionKind::Accessibility);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(2));
        let current = platform::services().permission_status(PermissionKind::Accessibility);
        if current != last {
            last = current;
            let _ = app.emit(
                "permission-changed",
                serde_json::json!({
                    "kind": "accessibility",
                    "status": match current {
                        PermissionStatus::Granted => "granted",
                        PermissionStatus::Denied => "denied",
                        PermissionStatus::NotDetermined => "not_determined",
                    },
                }),
            );
        }
    });
}

pub fn is_opaque() -> bool {
    slot()
        .lock()
        .map(|appearance| appearance.material == Material::Opaque)
        .unwrap_or(false)
}

pub fn window_manager_module() -> std::marker::PhantomData<()> {
    let _ = window_manager::POPUP_WIDTH;
    std::marker::PhantomData
}
