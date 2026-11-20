//! macOS implementation of `PlatformServices`.

use tauri::WebviewWindow;

use crate::platform::{
    Appearance, CaptureOutcome, PermissionKind, PermissionStatus, PlatformServices, Point, Rect,
};

pub mod appearance;
pub mod cursor;
pub mod events;
pub mod material;
pub mod panel;
pub mod pasteboard;
pub mod permissions;
pub mod selection;
pub mod tts;

pub struct MacosServices;

impl MacosServices {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacosServices {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformServices for MacosServices {
    fn capture_selection(&self) -> CaptureOutcome {
        selection::capture_selection()
    }

    fn cursor_position(&self) -> Point {
        cursor::cursor_position()
    }

    fn monitor_work_area(&self, point: Point) -> Rect {
        cursor::work_area_for(point)
    }

    fn permission_status(&self, kind: PermissionKind) -> PermissionStatus {
        permissions::status(kind)
    }

    fn request_permission(&self, kind: PermissionKind) {
        permissions::request(kind)
    }

    fn speak(&self, text: &str, lang: &str) -> Result<(), String> {
        tts::speak(text, lang)
    }

    fn voice_exists(&self, lang: &str) -> bool {
        tts::voice_exists(lang)
    }

    fn write_clipboard_text(&self, text: &str) -> Result<(), String> {
        pasteboard::write_text(text)
    }

    fn supports(&self, feature: &str) -> bool {
        matches!(
            feature,
            "selection" | "tts" | "accessibility" | "native_material"
        )
    }

    fn appearance(&self) -> Appearance {
        appearance::current_appearance()
    }

    fn apply_window_material(&self, window: &WebviewWindow, appearance: &Appearance) {
        material::apply(window, appearance);
    }

    fn show_without_activation(&self, window: &WebviewWindow) {
        panel::show_without_activation(window);
    }

    fn hide_window(&self, window: &WebviewWindow) {
        panel::hide(window);
    }

    fn read_clipboard_text(&self) -> Option<String> {
        pasteboard::read_text()
    }

    fn stop_speaking(&self) {
        tts::stop();
    }

    fn is_speaking(&self) -> bool {
        tts::is_speaking()
    }

    fn start_outside_click_monitor(&self, app: tauri::AppHandle) {
        events::start(app);
    }

    fn stop_outside_click_monitor(&self) {
        events::stop();
    }

    fn focus_popup(&self, window: &WebviewWindow) {
        // A non-activating panel can become key without activating Verso (R4).
        panel::focus_on_interaction(window);
    }

    fn restore_capture_target(&self) {
        // The panel never activates Verso, so AppKit already gives key back to the source app.
    }

    fn apply_settings_material(&self, window: &WebviewWindow, appearance: &Appearance) {
        material::apply_settings(window, appearance);
    }
}

/// Makes Verso the active app after it became a regular app, so the menu bar (and its key equivalents
/// such as Cmd+C, Cmd+V, Cmd+Q) is Verso's. Opening Settings from the non-activating quick panel left
/// the previous app's menu bar in place without it (spec 009 FR-053, S30). Main thread only.
pub fn activate_app() {
    let Some(mtm) = objc2::MainThreadMarker::new() else {
        return;
    };
    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
}
