//! macOS implementation of `PlatformServices`.

use tauri::WebviewWindow;

use crate::platform::{
    Appearance, CaptureOutcome, PermissionKind, PermissionStatus, PlatformServices, Point, Rect,
};

pub mod appearance;
pub mod cursor;
pub mod events;
pub mod material;
pub mod ocr;
pub mod panel;
pub mod pasteboard;
pub mod permissions;
pub mod screen_capture;
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

    fn reset_permission(&self, kind: PermissionKind) -> crate::platform::ResetOutcome {
        permissions::reset(kind)
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
        ) || (feature == "ocr" && ocr::availability().available)
    }

    fn ocr_availability(&self) -> crate::platform::OcrAvailability {
        ocr::availability()
    }

    fn capture_region(
        &self,
        region: crate::platform::CaptureRegion,
    ) -> Result<crate::platform::RegionImage, crate::platform::CaptureError> {
        // `CGPreflightScreenCaptureAccess` can stay stale after a revoke, so a refusal from
        // ScreenCaptureKit itself is mapped too (screen_capture.rs); this check avoids a prompt.
        if permissions::status(PermissionKind::ScreenRecording) != PermissionStatus::Granted {
            return Err(crate::platform::CaptureError::PermissionMissing);
        }
        screen_capture::capture(region)
    }

    fn recognise(
        &self,
        image: crate::platform::RegionImage,
        language: &str,
    ) -> Result<Vec<crate::platform::RecognisedLine>, crate::platform::OcrError> {
        ocr::recognise(image, language)
    }

    fn prepare_overlay(&self, window: &WebviewWindow) {
        panel::prepare_overlay(window);
    }

    fn show_overlay(&self, window: &WebviewWindow) {
        panel::show_overlay(window);
    }

    fn close_overlay(&self, window: &WebviewWindow) {
        panel::close_overlay(window);
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

    fn activate_for_typing(&self) {
        let own = std::process::id() as i32;
        let frontmost = objc2_app_kit::NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|app| app.processIdentifier());
        if let Some(pid) = frontmost.filter(|pid| *pid != own) {
            PREVIOUS_APP.store(pid, std::sync::atomic::Ordering::SeqCst);
        }
        activate_app();
    }

    fn end_typing(&self) {
        let Some(mtm) = objc2::MainThreadMarker::new() else {
            return;
        };
        let pid = PREVIOUS_APP.swap(0, std::sync::atomic::Ordering::SeqCst);
        if pid == 0 || !objc2_app_kit::NSApplication::sharedApplication(mtm).isActive() {
            return;
        }
        if let Some(app) =
            objc2_app_kit::NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        {
            app.activateWithOptions(objc2_app_kit::NSApplicationActivationOptions::empty());
        }
    }
}

/// The app that was active when the panel opened for typing (spec 004 FR-030); 0 when none.
static PREVIOUS_APP: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

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
