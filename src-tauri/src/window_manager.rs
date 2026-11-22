//! Window creation, the popup pre-warm, the lazy quick panel, placement and non-activating show
//! (popup-ui-contract.md).

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use log::{info, warn};
use tauri::{AppHandle, Emitter, Listener, LogicalPosition, LogicalSize, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::core::capture::PopupState;
use crate::core::quick_queue::QuickQueue;
use crate::platform::{self, PermissionKind, PermissionReason, Point, Rect, Size};

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

/// Height-only resize of a locked popup frame: `x`, `y` and the width never change; the height
/// is clamped to the space below the top edge and to the popup limits (R20).
pub fn locked_resize(frame: Rect, requested_height: f64, work_area: Rect) -> Size {
    let room = work_area.bottom() - frame.y - EDGE_MARGIN;
    let max = POPUP_MAX_HEIGHT.min(room).max(POPUP_MIN_HEIGHT);
    Size {
        width: frame.width,
        height: requested_height.clamp(POPUP_MIN_HEIGHT, max),
    }
}

pub fn popup_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("popup")
}

pub fn quick_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("quick")
}

pub fn quick_is_visible(app: &AppHandle) -> bool {
    quick_window(app)
        .map(|window| window.is_visible().unwrap_or(false))
        .unwrap_or(false)
}

/// Whether the quick panel is open for the user. It is never pre-warmed (T122), so a visible panel is
/// one the user opened.
pub fn quick_is_open(app: &AppHandle) -> bool {
    quick_is_visible(app)
}

/// Panel style and the blur rule for the popup and the quick panel: on macOS a non-activating
/// panel, on Windows a tool window; losing focus hides it (a pinned popup stays, R5).
pub fn prepare_transient(app: &AppHandle, window: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    platform::macos::panel::prepare(window);
    #[cfg(windows)]
    platform::windows::window_style::prepare(window);

    let handle = app.clone();
    let is_popup = window.label() == "popup";
    let watched = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            let visible = watched.is_visible().unwrap_or(false);
            if !is_popup {
                log::info!("quick panel lost focus (visible: {visible})");
                // A loading panel keeps its queued show; only a panel on screen closes (T134).
                if QUICK.lock().blur(visible) {
                    hide_quick(&handle);
                }
                return;
            }
            if !visible {
                return;
            }
            if POPUP_PINNED.load(Ordering::SeqCst) {
                return;
            }
            hide_popup(&handle);
        }
    });
}

/// Until the quick page has mounted its listeners (`quick_ready`), a show and a prefill wait here
/// and are replayed (T122; rules in `core::quick_queue`, T134).
static QUICK: parking_lot::Mutex<QuickQueue> = parking_lot::Mutex::new(QuickQueue::new());

/// The quick panel is built on its first open and then only hidden, so it costs no memory until
/// the user wants it (T122, Constitution I). The popup stays pre-warmed for the 150 ms rule.
fn ensure_quick(app: &AppHandle) -> Option<WebviewWindow> {
    if let Some(window) = quick_window(app) {
        return Some(window);
    }
    QUICK.lock().created();
    let builder =
        tauri::WebviewWindowBuilder::new(app, "quick", tauri::WebviewUrl::App("quick.html".into()))
            .title("Verso")
            .inner_size(QUICK_WIDTH, QUICK_HEIGHT)
            .visible(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .shadow(true);
    match builder.build() {
        Ok(window) => {
            // The first open may come from an async command; AppKit and Win32 styling must run on
            // the main thread. The page loads meanwhile, and the panel shows only after `quick_ready`.
            let handle = app.clone();
            let styled = window.clone();
            let _ = app.run_on_main_thread(move || {
                prepare_transient(&handle, &styled);
                let appearance = crate::appearance::current(&handle);
                platform::services().apply_window_material(&styled, &appearance);
            });
            info!("quick panel created on first use");
            // Safety net: if the page never reports ready (a script error), show it anyway.
            let late = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                if !QUICK.lock().is_ready() {
                    warn!("quick page did not report ready in 3 s; showing it anyway");
                    quick_page_ready(&late);
                }
            });
            Some(window)
        }
        Err(error) => {
            warn!("could not create the quick panel: {error}");
            None
        }
    }
}

/// The quick page reports that its listeners are in place: replay what waited for it.
pub fn quick_page_ready(app: &AppHandle) {
    let replay = QUICK.lock().page_ready();
    info!("quick page ready (queued show: {})", replay.present);
    if replay.present {
        if let Some(window) = quick_window(app) {
            let _ = present_quick(&window);
        }
    }
    if let Some(text) = replay.prefill {
        let _ = app.emit("quick-prefill", serde_json::json!({ "text": text }));
    }
}

/// Sends the clipboard prefill to the panel, or keeps it until the panel's page is ready.
pub fn send_quick_prefill(app: &AppHandle, text: String) {
    let now = QUICK.lock().prefill(text);
    if let Some(text) = now {
        let _ = app.emit("quick-prefill", serde_json::json!({ "text": text }));
    }
}

/// Shows the popup once off-screen so its webview is warm (SC-002).
pub fn prewarm(app: &AppHandle) {
    let Some(window) = popup_window(app) else {
        return;
    };
    let size = popup_pill_size();
    let _ = window.set_size(LogicalSize::new(size.width, size.height));
    let _ = window.set_position(LogicalPosition::new(-10_000.0, -10_000.0));
    if let Err(error) = window.show() {
        warn!("could not pre-warm the popup: {error}");
    }
    let _ = window.hide();
    let _ = window.set_position(LogicalPosition::new(0.0, 0.0));
}

/// The cursor point the popup was placed at; content growth re-runs placement from it (R2/R3).
static POPUP_ANCHOR: parking_lot::Mutex<Option<Point>> = parking_lot::Mutex::new(None);

/// The frame of a result popup once placed; later resizes only change the height (R20).
static POPUP_LOCKED: parking_lot::Mutex<Option<Rect>> = parking_lot::Mutex::new(None);

/// Set when the quick panel was hidden, so a tray click that caused that blur does not reopen it.
static QUICK_HIDDEN_AT: parking_lot::Mutex<Option<std::time::Instant>> =
    parking_lot::Mutex::new(None);

/// True when the panel was hidden less than 300 ms ago (the blur caused by the tray click itself).
pub fn quick_just_hidden() -> bool {
    (*QUICK_HIDDEN_AT.lock())
        .map(|at| at.elapsed() < std::time::Duration::from_millis(300))
        .unwrap_or(false)
}

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
    *POPUP_LOCKED.lock() = None;
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
    log::debug!(
        "popup resize requested {width}x{height} -> {}x{}",
        size.width,
        size.height
    );
    let services = platform::services();
    let is_pill = size.width <= POPUP_PILL_WIDTH;
    let locked = *POPUP_LOCKED.lock();
    if let (false, Some(frame)) = (is_pill, locked) {
        let work_area = services.monitor_work_area(Point {
            x: frame.x,
            y: frame.y,
        });
        let resized = locked_resize(frame, size.height, work_area);
        let _ = window.set_size(LogicalSize::new(resized.width, resized.height));
        *POPUP_LOCKED.lock() = Some(Rect {
            height: resized.height,
            ..frame
        });
        return;
    }
    let cursor = POPUP_ANCHOR
        .lock()
        .unwrap_or_else(|| services.cursor_position());
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
    if !is_pill {
        *POPUP_LOCKED.lock() = Some(Rect {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        });
    }
}

/// Whether the popup may take the keyboard focus: only while it is on screen and the quick panel is
/// not open. Results are broadcast to every page, so the hidden popup also sees the panel's.
pub fn popup_may_take_focus(popup_visible: bool, quick_open: bool) -> bool {
    popup_visible && !quick_open
}

/// Set when the popup asked to hide while a requested main window was still loading (T035).
static POPUP_HIDE_PENDING: AtomicBool = AtomicBool::new(false);

/// Whether the popup should stay until the main window it asked for appears: hiding it first hands
/// the foreground to the source app, and Windows then refuses to bring Settings forward (T035).
pub fn defer_popup_hide(main_requested_recently: bool, main_window_visible: bool) -> bool {
    main_requested_recently && !main_window_visible
}

/// Whether hiding the popup gives the foreground back to the app the selection came from: only when
/// the popup was on screen, and not to take it from the panel or a main window.
pub fn hand_back_to_source(
    popup_was_visible: bool,
    quick_open: bool,
    main_requested_recently: bool,
    main_window_focused: bool,
) -> bool {
    popup_was_visible && !quick_open && !main_requested_recently && !main_window_focused
}

fn any_main_window(app: &AppHandle, test: impl Fn(&WebviewWindow) -> bool) -> bool {
    MAIN_WINDOWS
        .iter()
        .filter_map(|label| app.get_webview_window(label))
        .any(|window| test(&window))
}

pub fn hide_popup(app: &AppHandle) {
    let requested_recently =
        main_window_keeps_focus(*MAIN_REQUESTED_AT.lock(), std::time::Instant::now());
    let was_visible = popup_window(app).is_some_and(|window| window.is_visible().unwrap_or(false));
    if was_visible
        && defer_popup_hide(
            requested_recently,
            any_main_window(app, |window| window.is_visible().unwrap_or(false)),
        )
    {
        // `show_ready_window` hides it once the main window is on screen and focused.
        POPUP_HIDE_PENDING.store(true, Ordering::SeqCst);
        return;
    }
    POPUP_HIDE_PENDING.store(false, Ordering::SeqCst);
    if let Some(window) = popup_window(app) {
        // AppKit's `orderOut` must run on the main thread (macOS 26 traps otherwise, seen with
        // `--self-test` and any capture that finishes on a worker); queueing keeps the order
        // relative to a later `show_popup`, which is queued the same way.
        let hide_window = window.clone();
        let _ = app.run_on_main_thread(move || {
            let _ = hide_window.hide();
            platform::services().hide_window(&hide_window);
        });
    }
    // Monitors live on the main run loop (macOS NSEvent); hide_popup can run on a worker thread.
    let monitor_handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = &monitor_handle;
        platform::services().stop_outside_click_monitor();
    });
    // The popup page also asks to hide itself when the capture found nothing, and that request can
    // arrive after the quick panel was opened for the same press; handing the foreground back to
    // the source window then would steal the panel's focus and close it (seen on Windows).
    // Likewise when a main window was just asked for from the popup (`Ctrl+,` / `⌘,` opens
    // Settings, then the page hides the popup): Settings keeps the foreground (FR-049, T143).
    if hand_back_to_source(
        was_visible,
        quick_is_open(app),
        requested_recently,
        any_main_window(app, |window| window.is_focused().unwrap_or(false)),
    ) {
        platform::services().restore_capture_target();
    }
    POPUP_PINNED.store(false, Ordering::SeqCst);
    *POPUP_ANCHOR.lock() = None;
    *POPUP_LOCKED.lock() = None;
    unregister_escape(app);
    let _ = app.emit("popup-state", PopupState::Hidden);
}

/// Opens the quick panel and gives it focus (the user is about to type).
pub fn show_quick(app: &AppHandle) -> tauri::Result<()> {
    if focus_setup_if_open(app) {
        return Ok(());
    }
    let Some(window) = ensure_quick(app) else {
        return Ok(());
    };
    // A new panel shows once its page is ready, never as a blank frame.
    let now = QUICK.lock().request_show();
    if !now {
        return Ok(());
    }
    present_quick(&window)
}

fn present_quick(window: &WebviewWindow) -> tauri::Result<()> {
    log::info!("quick panel: present");
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
    nudge_layout(window);
    // The focused panel handles Escape itself; only the popup needs the global registration (T056).
    Ok(())
}

pub fn hide_quick(app: &AppHandle) {
    QUICK.lock().hide();
    if let Some(window) = quick_window(app) {
        if window.is_visible().unwrap_or(false) {
            *QUICK_HIDDEN_AT.lock() = Some(std::time::Instant::now());
        }
        let _ = window.hide();
    }
    // Escape belongs to the popup: hiding the panel must not unregister it.
}

pub fn toggle_quick(app: &AppHandle) {
    let pending = QUICK.lock().show_pending();
    if quick_is_visible(app) || pending {
        hide_quick(app);
    } else {
        let _ = show_quick(app);
    }
}

/// Settings is created on first use, never at startup: every extra webview costs ~15–30 MB of
/// idle memory and the app must stay under the 150 MB budget (SC-003).
pub fn show_settings(app: &AppHandle) {
    note_main_requested();
    if focus_setup_if_open(app) {
        return;
    }
    if let Some(window) = app.get_webview_window("settings") {
        bring_to_front(app, &window);
        return;
    }
    open_settings(app, "settings.html");
}

/// Opens Settings at one of its sections (`about`): a new window serves the section to the page on
/// mount (the hash alone is not reliable across webviews), an open one receives `settings-section`
/// and scrolls there (T025).
pub fn show_settings_section(app: &AppHandle, section: &str) {
    note_main_requested();
    if focus_setup_if_open(app) {
        return;
    }
    if let Some(window) = app.get_webview_window("settings") {
        bring_to_front(app, &window);
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
    let builder =
        tauri::WebviewWindowBuilder::new(app, "settings", tauri::WebviewUrl::App(url.into()))
            .title(crate::ui_text::current(
                app,
                crate::ui_text::Text::SettingsTitle,
            ))
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
            // AppKit needs the main thread, and `⌘,` / the header Settings button arrive on a
            // worker thread through the async command (spec 010 R5); the user's theme override
            // applies from the first frame.
            let handle = app.clone();
            let styled = window.clone();
            let _ = app.run_on_main_thread(move || {
                let appearance = crate::appearance::current(&handle);
                platform::services().apply_settings_material(&styled, &appearance);
            });
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
            track_main_window(app, &window);
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
    note_main_requested();
    if focus_setup_if_open(app) {
        return;
    }
    *UPDATE_MODE.lock() = mode.to_string();
    if let Some(window) = app.get_webview_window("update") {
        bring_to_front(app, &window);
        let _ = window.emit("update-mode", mode);
        return;
    }
    let builder = tauri::WebviewWindowBuilder::new(
        app,
        "update",
        tauri::WebviewUrl::App("update.html".into()),
    )
    .title(crate::ui_text::current(
        app,
        crate::ui_text::Text::UpdateTitle,
    ))
    .inner_size(600.0, 460.0)
    .min_inner_size(600.0, 460.0)
    .resizable(false)
    .visible(false);
    match builder.build() {
        Ok(window) => {
            reveal_on_ready(app, &window);
            track_main_window(app, &window);
        }
        Err(error) => warn!("could not open the update window: {error}"),
    }
}

/// Renames the open main windows after the UI language changed.
pub fn retitle_main_windows(app: &AppHandle) {
    use crate::ui_text::{current, Text};
    for (label, key) in [
        ("settings", Text::SettingsTitle),
        ("update", Text::UpdateTitle),
        ("onboarding", Text::WelcomeTitle),
    ] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_title(current(app, key));
        }
    }
}

/// When a main window (Setup, Settings, Update) was last asked for; hiding the popup right after
/// must not hand the foreground back to the source app (T143).
static MAIN_REQUESTED_AT: parking_lot::Mutex<Option<std::time::Instant>> =
    parking_lot::Mutex::new(None);
const MAIN_FOCUS_GRACE: std::time::Duration = std::time::Duration::from_secs(3);

fn note_main_requested() {
    *MAIN_REQUESTED_AT.lock() = Some(std::time::Instant::now());
}

/// Whether a main window asked for at `requested` still owns the foreground at `now`.
pub fn main_window_keeps_focus(
    requested: Option<std::time::Instant>,
    now: std::time::Instant,
) -> bool {
    requested.is_some_and(|at| now.saturating_duration_since(at) < MAIN_FOCUS_GRACE)
}

/// Setup, Settings and the update dialog: the windows that give Verso a Dock icon (FR-053).
const MAIN_WINDOWS: [&str; 3] = ["onboarding", "settings", "update"];

/// While Setup is open nothing else opens: it comes to the front instead (FR-052).
pub fn focus_setup_if_open(app: &AppHandle) -> bool {
    let Some(window) = app.get_webview_window("onboarding") else {
        return false;
    };
    if !window.is_visible().unwrap_or(false) {
        return false;
    }
    bring_to_front(app, &window);
    true
}

fn bring_to_front(app: &AppHandle, window: &WebviewWindow) {
    note_main_requested();
    set_dock_icon(app, true);
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

/// When a main window is destroyed and no other one is shown, Verso goes back to the tray alone.
fn track_main_window(app: &AppHandle, window: &WebviewWindow) {
    let handle = app.clone();
    let label = window.label().to_string();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            // Main windows are destroyed when closed, so one that exists is open or about to show
            // (Setup closing Settings is created hidden until its page is ready, T136).
            let others = MAIN_WINDOWS
                .iter()
                .any(|other| *other != label && handle.get_webview_window(other).is_some());
            if !others {
                set_dock_icon(&handle, false);
            }
        }
    });
}

/// macOS: `Regular` (Dock icon, Cmd+Tab, menu bar) while a main window is open, `Accessory`
/// otherwise; the bundle starts as `LSUIElement`. Windows needs nothing: only the popup and the
/// quick panel skip the taskbar, and the main windows are destroyed when closed.
#[allow(unused_variables)]
fn set_dock_icon(app: &AppHandle, visible: bool) {
    #[cfg(target_os = "macos")]
    {
        let policy = if visible {
            tauri::ActivationPolicy::Regular
        } else {
            tauri::ActivationPolicy::Accessory
        };
        if let Err(error) = app.set_activation_policy(policy) {
            warn!("could not change the activation policy: {error}");
        }
        if visible {
            let _ = app.run_on_main_thread(platform::macos::activate_app);
        }
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
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        if window.is_visible().unwrap_or(false) {
            return;
        }
        if MAIN_WINDOWS.contains(&window.label()) {
            set_dock_icon(&app, true);
        }
        let _ = window.show();
        let _ = window.set_focus();
        nudge_layout(&window);
        // The popup that asked for this window stayed until now, so Verso still owned the
        // foreground when the window took the focus (T035).
        if MAIN_WINDOWS.contains(&window.label()) && POPUP_HIDE_PENDING.load(Ordering::SeqCst) {
            // Windows may report the focus a moment later; a fresh request time keeps the
            // foreground from going back to the source app as the popup hides.
            note_main_requested();
            hide_popup(&app);
        }
    });
}

/// 1-pt nudge: forces a layout pass on a webview that painted before being shown, so its first
/// frame is never blank (R13). Used by the main windows and the quick panel (T137).
fn nudge_layout(window: &WebviewWindow) {
    if let Ok(size) = window.inner_size() {
        let scale = window.scale_factor().unwrap_or(1.0);
        let logical = size.to_logical::<f64>(scale);
        let _ = window.set_size(LogicalSize::new(logical.width, logical.height - 1.0));
        let _ = window.set_size(LogicalSize::new(logical.width, logical.height));
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

/// Why the permission step is open, per permission kind (spec 010 data-model). Process memory only;
/// back to `Missing` when the setup window closes.
static PERMISSION_REASONS: parking_lot::Mutex<
    Option<std::collections::HashMap<PermissionKind, PermissionReason>>,
> = parking_lot::Mutex::new(None);

pub fn set_permission_reason(kind: PermissionKind, reason: PermissionReason) {
    PERMISSION_REASONS
        .lock()
        .get_or_insert_with(Default::default)
        .insert(kind, reason);
}

pub fn permission_reason(kind: PermissionKind) -> PermissionReason {
    PERMISSION_REASONS
        .lock()
        .as_ref()
        .and_then(|reasons| reasons.get(&kind).copied())
        .unwrap_or_default()
}

fn clear_permission_reasons() {
    *PERMISSION_REASONS.lock() = None;
}

/// Opens the onboarding directly at the Accessibility step, saying why: missing at runtime, or lost
/// across an update (spec 010 FR-003).
pub fn show_permission_onboarding(app: &AppHandle, reason: PermissionReason) {
    set_permission_reason(PermissionKind::Accessibility, reason);
    ONBOARDING_STEP.store(1, Ordering::SeqCst);
    show_onboarding(app);
}

pub fn show_onboarding(app: &AppHandle) {
    note_main_requested();
    // Setup takes over: it is the only main window while it is open (FR-052).
    for label in ["settings", "update"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.close();
        }
    }
    if let Some(window) = app.get_webview_window("onboarding") {
        bring_to_front(app, &window);
        return;
    }
    match tauri::WebviewWindowBuilder::new(
        app,
        "onboarding",
        tauri::WebviewUrl::App("onboarding.html".into()),
    )
    .title(crate::ui_text::current(
        app,
        crate::ui_text::Text::WelcomeTitle,
    ))
    .inner_size(560.0, 600.0)
    .resizable(false)
    .visible(false)
    .build()
    {
        Ok(window) => {
            // Same hidden-until-ready rule as Settings (R13).
            reveal_on_ready(app, &window);
            window.on_window_event(|event| {
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    clear_permission_reasons();
                }
            });
            track_main_window(app, &window);
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

    #[test]
    fn a_main_window_asked_for_just_now_keeps_the_foreground() {
        let now = std::time::Instant::now();
        assert!(!main_window_keeps_focus(None, now));
        assert!(main_window_keeps_focus(Some(now), now));
        let later = now + std::time::Duration::from_secs(5);
        assert!(!main_window_keeps_focus(Some(now), later));
    }

    #[test]
    fn the_popup_waits_for_a_requested_main_window_to_appear() {
        // T035: hiding the popup before Settings shows hands the foreground to the source app.
        assert!(defer_popup_hide(true, false));
        assert!(!defer_popup_hide(true, true));
        assert!(!defer_popup_hide(false, false));
    }

    #[test]
    fn no_hand_back_while_a_main_window_has_the_focus() {
        // T035: the source app must not take the foreground from Settings, however late the
        // popup hides.
        assert!(hand_back_to_source(true, false, false, false));
        assert!(!hand_back_to_source(true, true, false, false)); // quick panel open
        assert!(!hand_back_to_source(true, false, true, false)); // main window just asked for
        assert!(!hand_back_to_source(true, false, false, true)); // a main window is focused
    }

    #[test]
    fn a_hidden_popup_never_takes_the_focus() {
        // 009-S13 on Windows: the panel's result reached the hidden popup page, which asked for the
        // focus and took the foreground from the panel; the panel then closed on blur.
        assert!(popup_may_take_focus(true, false));
        assert!(!popup_may_take_focus(false, false));
        assert!(!popup_may_take_focus(true, true));
    }

    #[test]
    fn no_hand_back_when_the_popup_was_not_on_screen() {
        // A tray click hides the (already hidden) popup before it opens the panel; handing the
        // foreground to the last source app then took it from the new panel, which closed on blur
        // (009-S13 on Windows: the third click did not open the panel).
        assert!(!hand_back_to_source(false, false, false, false));
    }

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
    fn a_locked_popup_keeps_its_corner_and_width() {
        let frame = Rect {
            x: 300.0,
            y: 200.0,
            width: 380.0,
            height: 140.0,
        };
        let size = locked_resize(frame, 300.0, work_area());
        assert_eq!(size.width, 380.0);
        assert_eq!(size.height, 300.0);
    }

    #[test]
    fn a_locked_popup_height_is_clamped_to_the_space_below_and_the_limits() {
        let area = work_area();
        let low = Rect {
            x: 10.0,
            y: 700.0,
            width: 380.0,
            height: 140.0,
        };
        let size = locked_resize(low, 400.0, area);
        assert_eq!(size.height, area.bottom() - 700.0 - EDGE_MARGIN);
        let top = Rect { y: 20.0, ..low };
        assert_eq!(locked_resize(top, 5_000.0, area).height, POPUP_MAX_HEIGHT);
        assert_eq!(locked_resize(top, 10.0, area).height, POPUP_MIN_HEIGHT);
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
            Point {
                x: 1430.0,
                y: 890.0,
            },
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
