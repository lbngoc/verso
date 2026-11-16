//! Window creation, pre-warming, placement and non-activating show (popup-ui-contract.md).

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use log::{info, warn};
use tauri::{AppHandle, Emitter, Listener, LogicalPosition, LogicalSize, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::core::capture::PopupState;
use crate::platform::{self, Point, Rect, Size};

pub const POPUP_PILL_WIDTH: f64 = 200.0;
pub const POPUP_PILL_HEIGHT: f64 = 36.0;
pub const POPUP_WIDTH: f64 = 380.0;
pub const POPUP_WIDE_WIDTH: f64 = 560.0;
pub const POPUP_MIN_HEIGHT: f64 = 96.0;
pub const POPUP_MAX_HEIGHT: f64 = 420.0;
/// Default result height, used to place the window before the page reports its measured height.
pub const POPUP_RESULT_HEIGHT: f64 = 140.0;
pub const POPUP_MAX_WIDTH: f64 = 560.0;
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

/// The pill the popup starts as while translating (contract § Status tooltip).
pub fn popup_pill_size() -> Size {
    Size {
        width: POPUP_PILL_WIDTH,
        height: POPUP_PILL_HEIGHT,
    }
}

/// The size a result should use: wider for long text, then the page reports its measured height.
pub fn popup_result_size(text_len: usize, step: i8) -> Size {
    let width = if text_len > 240 || step > 1 {
        POPUP_WIDE_WIDTH
    } else {
        POPUP_WIDTH
    };
    let lines = (text_len as f64 / 48.0).ceil().max(1.0);
    let height = (32.0 + 36.0 + 10.0 + lines * 22.0).clamp(POPUP_MIN_HEIGHT, POPUP_MAX_HEIGHT);
    Size { width, height }
}

/// Clamps a size the page measured into what the popup may occupy (R3).
pub fn clamp_popup_size(width: f64, height: f64) -> Size {
    Size {
        width: width.clamp(POPUP_PILL_WIDTH, POPUP_MAX_WIDTH),
        height: height.clamp(POPUP_PILL_HEIGHT, POPUP_MAX_HEIGHT),
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
        ("popup", popup_pill_size()),
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

/// The cursor point the popup was placed at; content growth re-runs placement from it (R2/R3).
static POPUP_ANCHOR: parking_lot::Mutex<Option<Point>> = parking_lot::Mutex::new(None);

/// Pin state (memory only): a pinned popup ignores blur and outside clicks; every show resets it.
pub static POPUP_PINNED: AtomicBool = AtomicBool::new(false);

/// Shows the popup as the compact status pill at the cursor without activating it (FR-002).
pub fn show_popup(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = popup_window(app) else {
        return Ok(());
    };
    let services = platform::services();
    let cursor = services.cursor_position();
    let work_area = services.monitor_work_area(cursor);
    let size = popup_pill_size();
    let position = place_popup(&PlacementInput {
        cursor,
        work_area,
        size,
        scale_factor: 1.0,
        gap: PLACEMENT_GAP,
    });

    *POPUP_ANCHOR.lock() = Some(cursor);
    POPUP_PINNED.store(false, Ordering::SeqCst);
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
    // Unpinned: a click outside closes the popup even before it ever took focus (R5).
    platform::services().start_outside_click_monitor(app.clone());
    app.emit(
        "popup-state",
        PopupState::Status {
            kind: crate::core::capture::StatusKind::Translating,
        },
    )?;
    Ok(())
}

fn services_show(window: &WebviewWindow) {
    platform::services().show_without_activation(window);
}

/// Grows (or shrinks) the open popup to the size the page measured, re-placing it from the anchor.
pub fn resize_popup_to(app: &AppHandle, width: f64, height: f64) {
    let Some(window) = popup_window(app) else {
        return;
    };
    let size = clamp_popup_size(width, height);
    let services = platform::services();
    let cursor = POPUP_ANCHOR.lock().unwrap_or_else(|| services.cursor_position());
    let work_area = services.monitor_work_area(cursor);
    let position = place_popup(&PlacementInput {
        cursor,
        work_area,
        size,
        scale_factor: 1.0,
        gap: PLACEMENT_GAP,
    });
    let _ = window.set_size(LogicalSize::new(size.width, size.height));
    let _ = window.set_position(LogicalPosition::new(position.x, position.y));
}

pub fn hide_popup(app: &AppHandle) {
    if let Some(window) = popup_window(app) {
        let _ = window.hide();
        platform::services().hide_window(&window);
    }
    // Monitors live on the main run loop (macOS NSEvent); hide_popup can run on a worker thread.
    let monitor_handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = &monitor_handle;
        platform::services().stop_outside_click_monitor();
    });
    platform::services().restore_capture_target();    POPUP_PINNED.store(false, Ordering::SeqCst);
    *POPUP_ANCHOR.lock() = None;
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
    // The focused panel handles Escape itself; only the popup needs the global registration (T056).
    Ok(())
}

pub fn hide_quick(app: &AppHandle) {
    if let Some(window) = quick_window(app) {
        let _ = window.hide();
    }
    // Escape belongs to the popup: hiding the panel must not unregister it.
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
    let builder = tauri::WebviewWindowBuilder::new(app, "settings", tauri::WebviewUrl::App(url.into()))
        .title("Verso Settings")
        .inner_size(780.0, 560.0)
        .min_inner_size(640.0, 460.0)
        .resizable(true)
        .visible(false);
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .transparent(true);
    // Windows: Mica (or the solid fallback) shows through the sidebar column; the page paints the
    // content column opaque over it.
    #[cfg(windows)]
    let builder = builder.transparent(true);
    match builder.build() {
        Ok(window) => {
            // Painted before the window appears, so the first frame is never blank (R13).
            reveal_on_ready(app, &window);
            // The sidebar material sits behind the webview; the page leaves that column transparent.
            let appearance = platform::services().appearance();
            platform::services().apply_settings_material(&window, &appearance);
            // A recorder may have suspended the global shortcuts; closing Settings restores them.
            let handle = app.clone();
            window.on_window_event(move |event| {
                if matches!(
                    event,
                    tauri::WindowEvent::Destroyed | tauri::WindowEvent::CloseRequested { .. }
                ) {
                    let _ = crate::shortcuts::resume(&handle);
                }
            });
        }
        Err(error) => warn!("could not open Settings: {error}"),
    }
}

/// How the update window was opened: `check` from About, `offer` from the tray (spec 009 T050).
static UPDATE_MODE: parking_lot::Mutex<String> = parking_lot::Mutex::new(String::new());

pub fn take_update_mode() -> String {
    let mode = UPDATE_MODE.lock().clone();
    if mode.is_empty() {
        "check".into()
    } else {
        mode
    }
}

/// Opens (or focuses) the update dialog: 600 × 460, not resizable, created hidden (R14).
pub fn open_update_window(app: &AppHandle, mode: &str) {
    *UPDATE_MODE.lock() = mode.to_string();
    if let Some(window) = app.get_webview_window("update") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("update-mode", mode);
        return;
    }
    let builder = tauri::WebviewWindowBuilder::new(
        app,
        "update",
        tauri::WebviewUrl::App("update.html".into()),
    )
    .title("Software Update")
    .inner_size(600.0, 460.0)
    .min_inner_size(600.0, 460.0)
    .resizable(false)
    .visible(false);
    match builder.build() {
        Ok(window) => reveal_on_ready(app, &window),
        Err(error) => warn!("could not open the update window: {error}"),
    }
}

/// Shows a window created hidden once its page reports `page-ready`, or after 600 ms at the latest.
fn reveal_on_ready(app: &AppHandle, window: &WebviewWindow) {
    let listener_app = app.clone();
    let listener_window = window.clone();
    let _ = window.listen("page-ready", move |_| {
        show_ready_window(&listener_app, &listener_window);
    });
    let timeout_window = window.clone();
    let timeout_app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(600));
        if !timeout_window.is_visible().unwrap_or(false) {
            show_ready_window(&timeout_app, &timeout_window);
        }
    });
}

fn show_ready_window(app: &AppHandle, window: &WebviewWindow) {
    let window = window.clone();
    let _ = app.run_on_main_thread(move || {
        if window.is_visible().unwrap_or(false) {
            return;
        }
        let _ = window.show();
        let _ = window.set_focus();
        // 1-pt nudge: forces a layout pass on a webview that painted before being shown (R13).
        if let Ok(size) = window.inner_size() {
            let scale = window.scale_factor().unwrap_or(1.0);
            let logical = size.to_logical::<f64>(scale);
            let _ = window.set_size(LogicalSize::new(logical.width, logical.height - 1.0));
            let _ = window.set_size(LogicalSize::new(logical.width, logical.height));
        }
    });
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
    .visible(false)
    .build()
    {
        Ok(window) => {
            // Same hidden-until-ready rule as Settings (R13).
            reveal_on_ready(app, &window);
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
        if let Err(error) = show_popup(app) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn work_area() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
        }
    }

    #[test]
    fn the_pill_is_compact() {
        let pill = popup_pill_size();
        assert_eq!(pill.width, 200.0);
        assert_eq!(pill.height, 36.0);
    }

    #[test]
    fn the_result_grows_wider_for_long_text_or_large_text_size() {
        assert_eq!(popup_result_size(120, 0).width, POPUP_WIDTH);
        assert_eq!(popup_result_size(240, 1).width, POPUP_WIDTH);
        assert_eq!(popup_result_size(241, 0).width, POPUP_WIDE_WIDTH);
        assert_eq!(popup_result_size(120, 2).width, POPUP_WIDE_WIDTH);
    }

    #[test]
    fn the_result_height_stays_inside_its_limits() {
        let short = popup_result_size(10, 0);
        assert!(short.height >= POPUP_MIN_HEIGHT);
        let huge = popup_result_size(20_000, 0);
        assert_eq!(huge.height, POPUP_MAX_HEIGHT);
        let clamped = clamp_popup_size(10_000.0, 10_000.0);
        assert_eq!(clamped.width, POPUP_MAX_WIDTH);
        assert_eq!(clamped.height, POPUP_MAX_HEIGHT);
        let tiny = clamp_popup_size(10.0, 10.0);
        assert_eq!(tiny.width, POPUP_PILL_WIDTH);
        assert_eq!(tiny.height, POPUP_PILL_HEIGHT);
    }

    #[test]
    fn a_grown_popup_near_the_edges_stays_inside_the_work_area() {
        let area = work_area();
        let size = popup_result_size(400, 0);
        for cursor in [
            Point { x: 1430.0, y: 890.0 },
            Point { x: 1430.0, y: 20.0 },
            Point { x: 20.0, y: 890.0 },
            Point { x: 700.0, y: 450.0 },
        ] {
            let placed = place_popup(&PlacementInput {
                cursor,
                work_area: area,
                size,
                scale_factor: 1.0,
                gap: PLACEMENT_GAP,
            });
            assert!(
                placed.x >= area.x && placed.x + size.width <= area.right(),
                "x {placed:?} for cursor {cursor:?}"
            );
            assert!(
                placed.y >= area.y && placed.y + size.height <= area.bottom(),
                "y {placed:?} for cursor {cursor:?}"
            );
        }
    }
}
