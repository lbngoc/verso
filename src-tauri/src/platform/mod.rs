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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    Accessibility,
    ScreenRecording,
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
