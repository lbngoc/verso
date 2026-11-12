//! Window creation, pre-warming, placement and non-activating show (popup-ui-contract.md).

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use log::{info, warn};
use tauri::{AppHandle, Emitter, Listener, LogicalPosition, LogicalSize, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::core::capture::PopupState;
use crate::platform::{self, Point, Rect, Size};

pub const POPUP_WIDTH: f64 = 380.0;
pub const POPUP_HEIGHT: f64 = 180.0;
pub const POPUP_EXPANDED_WIDTH: f64 = 720.0;
pub const POPUP_EXPANDED_HEIGHT: f64 = 320.0;
pub const QUICK_WIDTH: f64 = 560.0;
pub const QUICK_HEIGHT: f64 = 320.0;
pub const PLACEMENT_GAP: f64 = 12.0;
pub const EDGE_MARGIN: f64 = 8.0;

/// Pure placement input in logical points (multi-monitor, mixed HiDPI aware).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementInput {
    pub cursor: Point,
    pub work_area: Rect,
    /// Logical size of the window. `scale_factor` is informational: both are logical points.
    pub size: Size,
    pub scale_factor: f64,
    pub gap: f64,
}

/// Places the popup below-right of the cursor, flipping above and clamping inside the work area.
pub fn place_popup(input: &PlacementInput) -> Point {
    let margin = EDGE_MARGIN;
    let min_x = input.work_area.x + margin;
    let max_x = (input.work_area.right() - input.size.width - margin).max(min_x);
    let min_y = input.work_area.y + margin;
    let max_y = (input.work_area.bottom() - input.size.height - margin).max(min_y);

    let mut x = input.cursor.x + input.gap;
    if x > max_x {
        // No room right: put the panel to the left of the cursor, then clamp.
        x = input.cursor.x - input.gap - input.size.width;
    }
    let x = x.clamp(min_x, max_x);

    let mut y = input.cursor.y + input.gap;
    if y > max_y {
        // No room below: flip above the cursor (contract § Placement).
        y = input.cursor.y - input.gap - input.size.height;
    }
    let y = y.clamp(min_y, max_y);

    Point { x, y }
}

fn popup_size(expanded: bool) -> Size {
    if expanded {
        Size {
            width: POPUP_EXPANDED_WIDTH,
            height: POPUP_EXPANDED_HEIGHT,
        }
    } else {
        Size {
            width: POPUP_WIDTH,
            height: POPUP_HEIGHT,
        }
    }
}

pub fn popup_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("popup")
}

pub fn quick_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("quick")
}

/// Shows each transient window once off-screen so the webview is warm (SC-002).
pub fn prewarm(app: &AppHandle) {
    for (label, size) in [
        ("popup", popup_size(false)),
        (
            "quick",
            Size {
                width: QUICK_WIDTH,
                height: QUICK_HEIGHT,
            },
        ),
    ] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_size(LogicalSize::new(size.width, size.height));
            let _ = window.set_position(LogicalPosition::new(-10_000.0, -10_000.0));
            if let Err(error) = window.show() {
                warn!("could not pre-warm {label}: {error}");
            }
            let _ = window.hide();
            let _ = window.set_position(LogicalPosition::new(0.0, 0.0));
        }
    }
}

/// Shows the popup at the cursor without activating it (FR-002) and reports `loading`.
pub fn show_popup(app: &AppHandle, expanded: bool) -> tauri::Result<()> {
    let Some(window) = popup_window(app) else {
        return Ok(());
    };
    let services = platform::services();
    let cursor = services.cursor_position();
    let work_area = services.monitor_work_area(cursor);
    let size = popup_size(expanded);
    let position = place_popup(&PlacementInput {
        cursor,
        work_area,
        size,
        scale_factor: 1.0,
        gap: PLACEMENT_GAP,
    });

    platform::services().remember_capture_target();
    window.set_size(LogicalSize::new(size.width, size.height))?;
    window.set_position(LogicalPosition::new(position.x, position.y))?;
    // `orderFrontRegardless`/`ShowWindow` touch the platform UI thread. Wait until the show has
    // actually run, otherwise a fast capture failure can hide the window before this queued show
    // executes and leave a stale popup on screen.
    let (sender, receiver) = std::sync::mpsc::channel();
    let show_window = window.clone();
    let _ = app.run_on_main_thread(move || {
        services_show(&show_window);
        let _ = sender.send(());
    });
    let _ = receiver.recv_timeout(std::time::Duration::from_millis(300));
    register_escape(app);
    app.emit("popup-state", PopupState::Loading)?;
    Ok(())
}

fn services_show(window: &WebviewWindow) {
    platform::services().show_without_activation(window);
}

/// Resizes the open popup when the user toggles Expand (contract: 380 → 720 pt wide).
pub fn resize_popup(app: &AppHandle, expanded: bool) {
    if let Some(window) = popup_window(app) {
        let size = popup_size(expanded);
        let _ = window.set_size(LogicalSize::new(size.width, size.height));
    }
}

pub fn hide_popup(app: &AppHandle) {
    if let Some(window) = popup_window(app) {
        let _ = window.hide();
        platform::services().hide_window(&window);
    }
    unregister_escape(app);
    let _ = app.emit("popup-state", PopupState::Hidden);
}

/// Opens the quick panel and gives it focus (the user is about to type).
pub fn show_quick(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = quick_window(app) else {
        return Ok(());
    };
    let services = platform::services();
    let cursor = services.cursor_position();
    let work_area = services.monitor_work_area(cursor);
    let size = Size {
        width: QUICK_WIDTH,
        height: QUICK_HEIGHT,
    };
    let position = place_popup(&PlacementInput {
        cursor,
        work_area,
        size,
        scale_factor: 1.0,
        gap: PLACEMENT_GAP,
    });
    window.set_size(LogicalSize::new(size.width, size.height))?;
    window.set_position(LogicalPosition::new(position.x, position.y))?;
    window.show()?;
    window.set_focus()?;
    register_escape(app);
    Ok(())
}

pub fn hide_quick(app: &AppHandle) {
    if let Some(window) = quick_window(app) {
        let _ = window.hide();
    }
    unregister_escape(app);
}

pub fn toggle_quick(app: &AppHandle) {
    if let Some(window) = quick_window(app) {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = show_quick(app);
        }
    }
}

/// Settings is created on first use, never at startup: every extra webview costs ~15–30 MB of
/// idle memory and the app must stay under the 150 MB budget (SC-003).
pub fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    open_settings(app, "settings.html");
}

/// Opens Settings at one of its sections (`about`): a new window serves the section to the page on
/// mount (the hash alone is not reliable across webviews), an open one receives `settings-section`
/// and scrolls there (T025).
pub fn show_settings_section(app: &AppHandle, section: &str) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("settings-section", section);
        return;
    }
    if let Ok(mut pending) = SETTINGS_SECTION.lock() {
        *pending = Some(section.to_string());
    }
    open_settings(app, &format!("settings.html#{section}"));
}

/// The section Settings must scroll to after it loads; the page takes it once on mount.
pub fn take_settings_section() -> Option<String> {
    SETTINGS_SECTION.lock().ok()?.take()
}

static SETTINGS_SECTION: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn open_settings(app: &AppHandle, url: &str) {
    match tauri::WebviewWindowBuilder::new(app, "settings", tauri::WebviewUrl::App(url.into()))
        .title("Verso Settings")
        .inner_size(720.0, 620.0)
        .min_inner_size(560.0, 460.0)
        .resizable(true)
        .visible(true)
        .build()
    {
        Ok(window) => {
            let _ = window.set_focus();
        }
        Err(error) => warn!("could not open Settings: {error}"),
    }
}

/// Escape is registered globally only while a transient window is open, so it closes the popup
/// even though the popup never takes focus.
static ESCAPE_REGISTERED: AtomicBool = AtomicBool::new(false);

fn register_escape(app: &AppHandle) {
    if ESCAPE_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    let result = app
        .global_shortcut()
        .on_shortcut("Escape", |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                hide_popup(app);
                hide_quick(app);
            }
        });
    if result.is_err() {
        ESCAPE_REGISTERED.store(false, Ordering::SeqCst);
        warn!("could not register Escape for the popup");
    }
}

fn unregister_escape(app: &AppHandle) {
    if ESCAPE_REGISTERED.swap(false, Ordering::SeqCst) {
        let _ = app.global_shortcut().unregister("Escape");
    }
}

/// 0 = engine choice (first run), 1 = permission guide. The onboarding route reads it on mount.
static ONBOARDING_STEP: AtomicU8 = AtomicU8::new(0);

pub fn onboarding_step() -> u8 {
    ONBOARDING_STEP.load(Ordering::SeqCst)
}

/// Opens the onboarding directly at the Accessibility step (permission missing at runtime).
pub fn show_permission_onboarding(app: &AppHandle) {
    ONBOARDING_STEP.store(1, Ordering::SeqCst);
    show_onboarding(app);
}

pub fn show_onboarding(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("onboarding") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    match tauri::WebviewWindowBuilder::new(
        app,
        "onboarding",
        tauri::WebviewUrl::App("onboarding.html".into()),
    )
    .title("Welcome to Verso")
    .inner_size(560.0, 560.0)
    .resizable(false)
    .visible(true)
    .build()
    {
        Ok(window) => {
            let _ = window.set_focus();
        }
        Err(error) => warn!("could not open onboarding: {error}"),
    }
}

/// `--self-test N`: measures shortcut → painted for the first show and N following shows (SC-002).
///
/// The popup UI emits `selftest-painted` after two animation frames whenever it receives
/// `popup-state = loading`; the harness matches each show with the next paint.
pub fn self_test(app: &AppHandle, iterations: u32) {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    let (sender, receiver) = mpsc::channel::<Instant>();
    let sender = Arc::new(sender);
    let _listener = {
        let sender = sender.clone();
        app.listen("selftest-painted", move |_event| {
            let _ = sender.send(Instant::now());
        })
    };

    // The popup webview loads asynchronously after pre-warm; give its listeners a moment so the
    // first `popup-state` event is not missed.
    std::thread::sleep(Duration::from_millis(2_500));
    let mut samples = Vec::new();
    for index in 0..iterations {
        // Drop any paint event left over from the previous show before timing this one.
        while receiver.try_recv().is_ok() {}
        let started = Instant::now();
        if let Err(error) = show_popup(app, false) {
            warn!("self-test could not show the popup: {error}");
            return;
        }
        match receiver.recv_timeout(Duration::from_secs(3)) {
            Ok(painted) => {
                let elapsed = painted.saturating_duration_since(started);
                samples.push(elapsed);
                info!(
                    "self-test {index}: popup painted in {} ms",
                    elapsed.as_millis()
                );
            }
            Err(_) => {
                // The first show can arrive before the webview has mounted its listeners (WKWebView
                // is slower than WebView2 to warm up); keep going so later shows can be measured.
                warn!("self-test {index}: no paint event within 3 s");
                hide_popup(app);
                std::thread::sleep(Duration::from_millis(300));
                continue;
            }
        }
        hide_popup(app);
        std::thread::sleep(Duration::from_millis(250));
    }

    if !samples.is_empty() {
        let mut millis: Vec<u128> = samples.iter().map(|s| s.as_millis()).collect();
        millis.sort_unstable();
        let median = millis[millis.len() / 2];
        info!(
            "self-test: first {} ms, median {} ms over {} shows",
            millis[0],
            median,
            millis.len()
        );
    }
}
