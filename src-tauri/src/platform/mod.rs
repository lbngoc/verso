//! OS services behind one trait (platform-contract.md); every OS-specific call lives in
//! `platform/<os>/`. Nothing above this layer knows which OS it runs on.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

/// How the selection text was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMethod {
    Accessibility,
    Copy,
    None,
}

/// What happened to the user's clipboard (FR-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardState {
    Untouched,
    Restored,
    PartiallyRestored,
    NotRestored,
}

/// Why capture failed (FR-018).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureFailure {
    NothingSelected,
    PermissionMissing,
    SecureField,
    TerminalTui,
    ElevatedWindow,
    Timeout,
}

/// Result of one capture attempt; drives the popup, the quick panel and FR-018 messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureOutcome {
    pub method: CaptureMethod,
    pub text: Option<String>,
    pub clipboard: ClipboardState,
    pub skipped_formats: Vec<String>,
    pub failure: Option<CaptureFailure>,
    pub frontmost_app: Option<String>,
    pub duration_ms: u64,
    /// False when the copy fallback never touched the clipboard (so it stays untouched).
    pub copy_attempted: bool,
}

impl CaptureOutcome {
    pub fn success(method: CaptureMethod, text: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            method,
            text: Some(text.into()),
            clipboard: if method == CaptureMethod::Accessibility {
                ClipboardState::Untouched
            } else {
                ClipboardState::Restored
            },
            skipped_formats: Vec::new(),
            failure: None,
            frontmost_app: None,
            duration_ms,
            copy_attempted: method == CaptureMethod::Copy,
        }
    }

    pub fn failure(
        failure: CaptureFailure,
        frontmost_app: Option<String>,
        duration_ms: u64,
    ) -> Self {
        Self {
            method: CaptureMethod::None,
            text: None,
            clipboard: ClipboardState::Untouched,
            skipped_formats: Vec::new(),
            failure: Some(failure),
            frontmost_app,
            duration_ms,
            copy_attempted: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.right()
            && point.y >= self.y
            && point.y <= self.bottom()
    }
}

/// One line of text a recogniser found: its top candidate and its box (top-left origin, in the
/// pixels of the image that was recognised).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecognisedLine {
    pub text: String,
    pub rect: Rect,
}

/// Why the screen translate module cannot be used on this system (spec 004 FR-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcrUnavailableReason {
    OsTooOld,
    NoLanguage,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcrLanguage {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcrAvailability {
    pub available: bool,
    pub reason: Option<OcrUnavailableReason>,
    pub languages: Vec<OcrLanguage>,
}

impl OcrAvailability {
    pub fn unsupported() -> Self {
        Self {
            available: false,
            reason: Some(OcrUnavailableReason::Unsupported),
            languages: Vec::new(),
        }
    }
}

/// The screen region to read: the same area in global logical points (macOS) and in physical
/// pixels of the virtual screen (Windows), plus the scale of the monitor it lies on.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub logical: Rect,
    pub physical: Rect,
    pub scale: f64,
}

/// A captured region. The payload is platform specific, is never serialised or cloned, and is
/// dropped right after recognition: images stay in memory only (spec 004 FR-006).
pub struct RegionImage {
    payload: Box<dyn std::any::Any + Send>,
}

impl RegionImage {
    pub fn new<T: std::any::Any + Send>(payload: T) -> Self {
        Self {
            payload: Box::new(payload),
        }
    }

    /// The platform's own payload back, or `None` when it is another platform's.
    pub fn into_payload<T: std::any::Any>(self) -> Option<T> {
        self.payload.downcast::<T>().ok().map(|boxed| *boxed)
    }
}

impl std::fmt::Debug for RegionImage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RegionImage(..)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    /// Screen Recording is not (or no longer) granted: the permission step handles it.
    PermissionMissing,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrError(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    Accessibility,
    ScreenRecording,
}

/// Why the permission step is shown (spec 010): a grant lost across an update gets its own wording
/// and the one-click reset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionReason {
    #[default]
    Missing,
    AfterUpdate,
}

/// Result of "Reset and grant again": `ok` is false when the reset was refused (grant already
/// effective, unsupported platform) or the system tool failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetOutcome {
    pub ok: bool,
    pub status: PermissionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Os {
    Macos,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    Glass,
    Vibrancy,
    Acrylic,
    Opaque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiTheme {
    Light,
    Dark,
}

/// Appearance value computed at start and on OS change events (research P9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Appearance {
    pub os: Os,
    pub os_version: String,
    pub material: Material,
    pub theme: UiTheme,
    pub reduce_transparency: bool,
    pub high_contrast: bool,
    pub accent: Option<String>,
}

impl Appearance {
    pub fn opaque_fallback(mut self) -> Self {
        self.material = Material::Opaque;
        self
    }
}

/// Platform contract: all OS code lives behind this trait (constitution III).
pub trait PlatformServices: Send + Sync {
    fn capture_selection(&self) -> CaptureOutcome;
    fn cursor_position(&self) -> Point;
    fn monitor_work_area(&self, point: Point) -> Rect;
    fn permission_status(&self, kind: PermissionKind) -> PermissionStatus;
    fn request_permission(&self, kind: PermissionKind);
    /// Clears the app's own entry for `kind` and asks again (spec 010 R2). Platforms without such
    /// an entry (Windows) refuse and report the current status.
    fn reset_permission(&self, kind: PermissionKind) -> ResetOutcome {
        ResetOutcome {
            ok: false,
            status: self.permission_status(kind),
        }
    }
    fn speak(&self, text: &str, lang: &str) -> Result<(), String>;
    fn voice_exists(&self, lang: &str) -> bool;
    /// Replaces the clipboard with plain text (used by "Copy diagnostics").
    fn write_clipboard_text(&self, text: &str) -> Result<(), String>;
    fn supports(&self, feature: &str) -> bool;
    fn appearance(&self) -> Appearance;
    fn apply_window_material(&self, window: &tauri::WebviewWindow, appearance: &Appearance);
    fn show_without_activation(&self, window: &tauri::WebviewWindow);
    /// Hides the window natively: the platform show bypasses Tauri's visibility tracking.
    fn hide_window(&self, window: &tauri::WebviewWindow);
    /// Remembers the window to capture from before the popup is shown (Windows needs it: the
    /// popup can become the foreground window, which would hide the user's selection from UIA).
    fn remember_capture_target(&self) {}

    // ---- spec 009 additions (all default to a no-op so every platform and the test double compile). ----

    /// Plain text from the clipboard, read only when the user enabled the panel prefill (FR-011).
    fn read_clipboard_text(&self) -> Option<String> {
        None
    }

    /// Stops the current utterance, if any (the Speak shortcut pressed again).
    fn stop_speaking(&self) {}

    fn is_speaking(&self) -> bool {
        false
    }

    /// Watches for a mouse-down outside the popup while it is visible and unpinned (R5).
    fn start_outside_click_monitor(&self, _app: tauri::AppHandle) {}

    fn stop_outside_click_monitor(&self) {}

    /// Makes the popup key on hover without activating Verso (R4).
    fn focus_popup(&self, _window: &tauri::WebviewWindow) {}

    /// Gives key back to the app the selection came from when the popup orders out.
    fn restore_capture_target(&self) {}

    /// Draws the Settings sidebar material (macOS sidebar vibrancy, Windows Mica).
    fn apply_settings_material(&self, _window: &tauri::WebviewWindow, _appearance: &Appearance) {}

    // ---- spec 004: screen translate. Every method defaults to "unsupported". ----

    fn ocr_availability(&self) -> OcrAvailability {
        OcrAvailability::unsupported()
    }

    /// Copies a screen region into memory, leaving Verso's own windows out.
    fn capture_region(&self, _region: CaptureRegion) -> Result<RegionImage, CaptureError> {
        Err(CaptureError::Failed(
            "screen capture is not supported".into(),
        ))
    }

    /// Reads the text in a captured region. `language` is `"auto"` or a BCP-47 code.
    fn recognise(
        &self,
        _image: RegionImage,
        _language: &str,
    ) -> Result<Vec<RecognisedLine>, OcrError> {
        Err(OcrError("text recognition is not supported".into()))
    }

    /// Makes an overlay window cover the screen above everything else, without taking the app
    /// the user is in out of focus.
    fn prepare_overlay(&self, _window: &tauri::WebviewWindow) {}

    /// Puts the overlay on screen (it was created hidden).
    fn show_overlay(&self, window: &tauri::WebviewWindow) {
        let _ = window.show();
    }

    /// Closes an overlay window (a panel on macOS must be turned back into a window first).
    fn close_overlay(&self, window: &tauri::WebviewWindow) {
        let _ = window.destroy();
    }

    /// Whether the overlay windows are kept out of screen captures by the OS.
    fn overlay_excluded_from_capture(&self) -> bool {
        true
    }
}

static SERVICES: OnceLock<Box<dyn PlatformServices>> = OnceLock::new();

pub fn install(services: Box<dyn PlatformServices>) {
    let _ = SERVICES.set(services);
}

pub fn services() -> &'static dyn PlatformServices {
    SERVICES
        .get()
        .map(|services| services.as_ref())
        .expect("platform services are installed during setup")
}

#[cfg(target_os = "macos")]
pub fn native_services() -> Box<dyn PlatformServices> {
    Box::new(macos::MacosServices::new())
}

#[cfg(windows)]
pub fn native_services() -> Box<dyn PlatformServices> {
    Box::new(windows::WindowsServices::new())
}

/// Builds the platform service set and installs it for the app.
pub fn init() -> &'static dyn PlatformServices {
    install(native_services());
    services()
}
