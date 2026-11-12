//! Windows implementation of `PlatformServices`.

use tauri::WebviewWindow;

use crate::platform::{
    Appearance, CaptureOutcome, PermissionKind, PermissionStatus, PlatformServices, Point, Rect,
};

pub mod appearance;
pub mod clipboard;
pub mod cursor;
pub mod material;
pub mod selection;
pub mod tts;
pub mod window_style;

pub struct WindowsServices;

impl WindowsServices {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsServices {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformServices for WindowsServices {
    fn capture_selection(&self) -> CaptureOutcome {
        selection::capture_selection()
    }

    fn cursor_position(&self) -> Point {
        cursor::cursor_position()
    }

    fn monitor_work_area(&self, point: Point) -> Rect {
        cursor::work_area_for(point)
    }

    fn permission_status(&self, _kind: PermissionKind) -> PermissionStatus {
        // Windows needs no permission for UIA/clipboard capture.
        PermissionStatus::Granted
    }

    fn request_permission(&self, _kind: PermissionKind) {}

    fn speak(&self, text: &str, lang: &str) -> Result<(), String> {
        tts::speak(text, lang)
    }

    fn voice_exists(&self, lang: &str) -> bool {
        tts::voice_exists(lang)
    }

    fn write_clipboard_text(&self, text: &str) -> Result<(), String> {
        clipboard::write_text(text)
    }

    fn supports(&self, feature: &str) -> bool {
        matches!(feature, "selection" | "tts" | "native_material")
    }

    fn appearance(&self) -> Appearance {
        appearance::current_appearance()
    }

    fn apply_window_material(&self, window: &WebviewWindow, appearance: &Appearance) {
        material::apply(window, appearance);
    }

    fn show_without_activation(&self, window: &WebviewWindow) {
        window_style::show_without_activation(window);
    }

    fn hide_window(&self, window: &WebviewWindow) {
        window_style::hide(window);
    }

    fn remember_capture_target(&self) {
        selection::remember_capture_target();
    }
}
