//! Window creation, the popup pre-warm, the lazy quick panel, placement and non-activating show
//! (popup-ui-contract.md).

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use log::{debug, info, warn};
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

/// Places the popup next to a screen region (spec 004 R7): below it, aligned to its left edge;
/// else above; else to its right; else to its left; else inside its top. Always clamped into the
/// work area minus `EDGE_MARGIN`. Pure; all values are logical points.
pub fn place_near_region(region: Rect, work_area: Rect, size: Size, gap: f64) -> Point {
    let margin = EDGE_MARGIN;
    let min_x = work_area.x + margin;
    let max_x = (work_area.right() - size.width - margin).max(min_x);
    let min_y = work_area.y + margin;
    let max_y = (work_area.bottom() - size.height - margin).max(min_y);

    let aligned_x = region.x.clamp(min_x, max_x);
    let below = region.bottom() + gap;
    if below <= max_y {
        return Point {
            x: aligned_x,
            y: below,
        };
    }
    let above = region.y - gap - size.height;
    if above >= min_y {
        return Point {
            x: aligned_x,
            y: above,
        };
    }
    // Neither fits vertically: beside the region, vertically centred on it and clamped.
    let beside_y = (region.y + region.height / 2.0 - size.height / 2.0).clamp(min_y, max_y);
    let right = region.right() + gap;
    if right <= max_x {
        return Point {
            x: right,
            y: beside_y,
        };
    }
    let left = region.x - gap - size.width;
    if left >= min_x {
        return Point {
            x: left,
            y: beside_y,
        };
    }
    // Nothing fits (the region fills the screen): over its lower part, where a result is read.
    Point {
        x: aligned_x,
        y: (region.bottom() - size.height).clamp(min_y, max_y),
    }
}

/// Whether opening the panel records the app in front as the capture target: from the keyboard
/// the user's app is in front; a tray click puts the taskbar there (nothing worth returning to),
/// and a screen result keeps the target taken when the capture started.
pub fn records_capture_target(anchor: PanelAnchor) -> bool {
    matches!(anchor, PanelAnchor::Shortcut)
}

/// What a panel is placed against (Revision 2, FR-024, R20).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanelAnchor {
    /// The tray / menu-bar icon the user clicked (logical points, global).
    TrayIcon(Rect),
    /// A keyboard shortcut, the tray menu or anything else without a spot on screen.
    Shortcut,
    /// A screen region the user selected.
    Region(Rect),
}

/// How a screen result is anchored: beside the selected region. To try the tray or shortcut rule
/// instead (owner choice after trying 0.1.9, R20), return `PanelAnchor::Shortcut` here or the
/// remembered tray icon in `screen_result_anchor`; `placement.rs` pins each rule.
pub const SCREEN_RESULT_ANCHOR_RULE: ScreenResultRule = ScreenResultRule::Region;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenResultRule {
    Region,
    TrayIcon,
    Shortcut,
}

/// Places the quick panel for how it was opened. Pure; logical points; always inside the work
/// area minus `EDGE_MARGIN`.
pub fn place_panel(anchor: PanelAnchor, work_area: Rect, size: Size, gap: f64) -> Point {
    let min_x = work_area.x + EDGE_MARGIN;
    let max_x = (work_area.right() - size.width - EDGE_MARGIN).max(min_x);
    let min_y = work_area.y + EDGE_MARGIN;
    let max_y = (work_area.bottom() - size.height - EDGE_MARGIN).max(min_y);
    match anchor {
        PanelAnchor::Region(region) => place_near_region(region, work_area, size, gap),
        PanelAnchor::Shortcut => Point {
            x: (work_area.x + (work_area.width - size.width) / 2.0).clamp(min_x, max_x),
            y: (work_area.y + (work_area.height - size.height) / 3.0).clamp(min_y, max_y),
        },
        PanelAnchor::TrayIcon(icon) => {
            let centre_x = icon.x + icon.width / 2.0;
            let centre_y = icon.y + icon.height / 2.0;
            let beside_y = (centre_y - size.height / 2.0).clamp(min_y, max_y);
            let across_x = (centre_x - size.width / 2.0).clamp(min_x, max_x);
            // The work area excludes the menu bar and the taskbar, so where the icon sits relative
            // to it says which edge the bar is on; an auto-hidden bar falls back to the halves.
            if icon.right() <= work_area.x + 1.0 {
                Point {
                    x: work_area.x + gap,
                    y: beside_y,
                }
            } else if icon.x >= work_area.right() - 1.0 {
                Point {
                    x: (work_area.right() - gap - size.width).max(min_x),
                    y: beside_y,
                }
            } else if centre_y < work_area.y + work_area.height / 2.0 {
                Point {
                    x: across_x,
                    y: work_area.y + gap,
                }
            } else {
                Point {
                    x: across_x,
                    y: (work_area.bottom() - gap - size.height).max(min_y),
                }
            }
        }
    }
}

/// The corner a selection popup keeps from its pill to its result (FR-022, R17): its top-left
/// when it sits below the cursor, its bottom-left when it flipped above (then it grows upward).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupSlot {
    pub corner: Point,
    pub grow_up: bool,
}

/// Chooses the slot with the default result size, so the pill is already where the result will be.
pub fn popup_slot(input: &PlacementInput) -> PopupSlot {
    let position = place_popup(input);
    let grow_up = position.y + 0.001 < input.cursor.y;
    PopupSlot {
        corner: Point {
            x: position.x,
            y: if grow_up {
                position.y + input.size.height
            } else {
                position.y
            },
        },
        grow_up,
    }
}

/// The frame of a popup of `size` hanging from `slot`, kept inside the work area.
pub fn frame_in_slot(slot: PopupSlot, size: Size, work_area: Rect) -> Rect {
    let min_x = work_area.x + EDGE_MARGIN;
    let max_x = (work_area.right() - size.width - EDGE_MARGIN).max(min_x);
    let min_y = work_area.y + EDGE_MARGIN;
    let max_y = (work_area.bottom() - size.height - EDGE_MARGIN).max(min_y);
    let y = if slot.grow_up {
        slot.corner.y - size.height
    } else {
        slot.corner.y
    };
    Rect {
        x: slot.corner.x.clamp(min_x, max_x),
        y: y.clamp(min_y, max_y),
        width: size.width,
        height: size.height,
    }
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

/// The height a placed result may take: below its top edge when it hangs down, above its bottom
/// edge when it grows up from a flipped slot (code review: a flipped popup was capped by the room
/// below it and could hardly grow).
pub fn slot_resize(slot: PopupSlot, frame: Rect, requested_height: f64, work_area: Rect) -> Size {
    if !slot.grow_up {
        return locked_resize(frame, requested_height, work_area);
    }
    let room = slot.corner.y - work_area.y - EDGE_MARGIN;
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
                // A loading panel keeps its queued show; only a panel on screen closes (T134). A
                // capture still being read keeps its panel (a system prompt may take the focus).
                // Only a blur of the panel on screen counts as the user leaving it; a hidden,
                // loading panel can also report one.
                let held = if visible {
                    READING.lock().blur()
                } else {
                    READING.lock().is_reading()
                };
                if held {
                    // Remembered: if the panel has not got the focus back when reading ends, it
                    // closes then (code review: it used to stay on screen without focus).
                    return;
                }
                if QUICK.lock().blur(visible) {
                    hide_quick_keep_focus(&handle);
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
pub fn ensure_quick(app: &AppHandle) -> Option<WebviewWindow> {
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

/// The quick page reports that its listeners are in place: replay what waited for it. A queued
/// screen event goes before the show, so the panel never paints the previous content (FR-021).
pub fn quick_page_ready(app: &AppHandle) {
    let replay = QUICK.lock().page_ready();
    info!(
        "quick page ready (queued show: {}, screen events: {})",
        replay.present,
        replay.screen.len()
    );
    for event in replay.screen {
        let _ = app.emit("quick-screen", event);
    }
    if replay.present {
        if let Some(window) = quick_window(app) {
            let anchor = *QUICK_ANCHOR.lock();
            let _ = present_quick(&window, anchor);
        }
    }
    if let Some(text) = replay.prefill {
        let _ = app.emit("quick-prefill", serde_json::json!({ "text": text }));
    }
}

/// How the next panel show is anchored; kept for a show that waits for the page.
static QUICK_ANCHOR: parking_lot::Mutex<PanelAnchor> =
    parking_lot::Mutex::new(PanelAnchor::Shortcut);

/// The panel currently shows a screen result: closing it hands focus back (FR-023).
static QUICK_SCREEN_MODE: AtomicBool = AtomicBool::new(false);

/// The panel page still holds the last capture's state; a normal open resets it first.
static QUICK_SCREEN_STALE: AtomicBool = AtomicBool::new(false);

/// The reading phase of the current capture: a blur is held while reading, a close abandons it.
static READING: parking_lot::Mutex<crate::core::quick_queue::ReadingTracker> =
    parking_lot::Mutex::new(crate::core::quick_queue::ReadingTracker::new());

/// The tray icon's frame at the last click, for the tray variant of the screen-result rule.
static LAST_TRAY_ICON: parking_lot::Mutex<Option<Rect>> = parking_lot::Mutex::new(None);

pub fn remember_tray_icon(rect: Rect) {
    *LAST_TRAY_ICON.lock() = Some(rect);
}

fn screen_result_anchor(region: Rect) -> PanelAnchor {
    match SCREEN_RESULT_ANCHOR_RULE {
        ScreenResultRule::Region => PanelAnchor::Region(region),
        ScreenResultRule::TrayIcon => LAST_TRAY_ICON
            .lock()
            .map_or(PanelAnchor::Shortcut, PanelAnchor::TrayIcon),
        ScreenResultRule::Shortcut => PanelAnchor::Shortcut,
    }
}

/// Sends a screen event to the panel, or keeps it until the panel's page is ready.
pub fn send_quick_screen(app: &AppHandle, event: crate::core::quick_queue::QuickScreenEvent) {
    // The capture is "being read" from its reading event until a text or failure event arrives.
    match event.phase {
        "reading" => READING.lock().start(),
        "reset" => {}
        _ => {
            let focused = quick_window(app)
                .map(|window| window.is_focused().unwrap_or(false))
                .unwrap_or(false);
            let end = READING.lock().finish(focused);
            match end {
                crate::core::quick_queue::ReadingEnd::Deliver => {}
                crate::core::quick_queue::ReadingEnd::Drop => {
                    info!("capture dropped: the panel was closed while reading");
                    return;
                }
                crate::core::quick_queue::ReadingEnd::DeliverAndFocus => {
                    // The focus went away while reading (a system prompt, a click elsewhere): the
                    // result is shown and the panel takes the focus again, so the next click
                    // outside closes it as usual.
                    info!("panel lost the focus while reading; giving it back with the result");
                    if let Some(window) = quick_window(app) {
                        let _ = window.set_focus();
                    }
                }
            }
        }
    }
    let now = QUICK.lock().screen_event(event);
    if let Some(event) = now {
        let _ = app.emit("quick-screen", event);
    }
}

/// True while the panel is showing a screen result.
pub fn quick_in_screen_mode() -> bool {
    QUICK_SCREEN_MODE.load(Ordering::SeqCst)
}

/// Shows the panel for a captured region: the page resets to "Reading the text…" first, the show
/// waits for its acknowledgement (bounded), and the panel is placed by the screen-result rule
/// (FR-021, FR-024).
pub fn prepare_quick_screen(app: &AppHandle, region: Rect) {
    let Some(window) = ensure_quick(app) else {
        return;
    };
    let settings = app.state::<crate::commands::AppState>().settings.get();
    let (seq, prepared) = QUICK_SCREEN_ACK.lock().arm();
    QUICK_SCREEN_MODE.store(true, Ordering::SeqCst);
    QUICK_SCREEN_STALE.store(true, Ordering::SeqCst);
    let anchor = screen_result_anchor(region);
    *QUICK_ANCHOR.lock() = anchor;
    let event = crate::core::quick_queue::QuickScreenEvent::reading(settings.target_lang, seq);
    if !QUICK.lock().is_ready() {
        // The page is still loading (first capture after launch, a quick drag): the reading event
        // and the show wait in the queue and `quick_page_ready` replays them in that order, so the
        // panel never shows as a blank frame (code review).
        send_quick_screen(app, event);
        QUICK.lock().request_show();
        info!("panel page still loading; its show waits for the page");
        return;
    }
    send_quick_screen(app, event);
    if prepared
        .recv_timeout(std::time::Duration::from_millis(300))
        .is_err()
    {
        warn!("panel did not confirm its reading state in 300 ms; showing it anyway");
    }
    let _ = present_quick(&window, anchor);
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

/// The corner the popup keeps from its pill to its result (FR-022); growth re-runs placement from it.
static POPUP_SLOT: parking_lot::Mutex<Option<PopupSlot>> = parking_lot::Mutex::new(None);

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

/// Acknowledgements from the pages, numbered per show (`core::ack`, FR-021, FR-022).
static POPUP_ACK: parking_lot::Mutex<crate::core::ack::AckGate> =
    parking_lot::Mutex::new(crate::core::ack::AckGate::new());
static QUICK_SCREEN_ACK: parking_lot::Mutex<crate::core::ack::AckGate> =
    parking_lot::Mutex::new(crate::core::ack::AckGate::new());

/// The popup page has rendered the reset state of show `seq` (`popup_prepared`).
pub fn popup_prepared(seq: u64) {
    if !POPUP_ACK.lock().fire(seq) {
        debug!("popup acknowledgement {seq} is stale; ignored");
    }
}

/// The panel page has rendered the reading state of show `seq` (`quick_screen_prepared`).
pub fn quick_screen_prepared(seq: u64) {
    if !QUICK_SCREEN_ACK.lock().fire(seq) {
        debug!("panel acknowledgement {seq} is stale; ignored");
    }
}

/// Shows the popup as the compact status pill at the cursor without activating it (FR-002). The
/// page renders the reset "Translating" state first so the previous result is never painted
/// (FR-022).
pub fn show_popup(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = popup_window(app) else {
        return Ok(());
    };
    let services = platform::services();
    let cursor = services.cursor_position();
    let work_area = services.monitor_work_area(cursor);
    let slot = popup_slot(&PlacementInput {
        cursor,
        work_area,
        size: Size {
            width: POPUP_WIDTH,
            height: POPUP_RESULT_HEIGHT,
        },
        scale_factor: 1.0,
        gap: PLACEMENT_GAP,
    });
    let frame = frame_in_slot(slot, popup_pill_size(), work_area);

    *POPUP_SLOT.lock() = Some(slot);
    *POPUP_LOCKED.lock() = None;
    POPUP_PINNED.store(false, Ordering::SeqCst);
    platform::services().remember_capture_target();
    window.set_size(LogicalSize::new(frame.width, frame.height))?;
    window.set_position(LogicalPosition::new(frame.x, frame.y))?;
    // Reset first, show after the page answered: the previous result is never painted (FR-022).
    let (seq, prepared) = POPUP_ACK.lock().arm();
    app.emit(
        "popup-state",
        PopupState::Status {
            kind: crate::core::capture::StatusKind::Translating,
        },
    )?;
    // Sent after the state, so the page answers only once that state is rendered (`seq` lets a
    // late answer from an earlier show be told apart).
    app.emit("popup-prepare", serde_json::json!({ "seq": seq }))?;
    if prepared
        .recv_timeout(std::time::Duration::from_millis(150))
        .is_err()
    {
        warn!("popup did not confirm its reset state in 150 ms; showing it anyway");
    }
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
    let Some(slot) = *POPUP_SLOT.lock() else {
        return;
    };
    let is_pill = size.width <= POPUP_PILL_WIDTH;
    let work_area = services.monitor_work_area(slot.corner);
    let locked = *POPUP_LOCKED.lock();
    // After the first result only the height changes (R20); the corner of the slot never moves.
    let size = match (is_pill, locked) {
        (false, Some(frame)) => slot_resize(slot, frame, size.height, work_area),
        _ => size,
    };
    let frame = frame_in_slot(slot, size, work_area);
    let _ = window.set_size(LogicalSize::new(frame.width, frame.height));
    let _ = window.set_position(LogicalPosition::new(frame.x, frame.y));
    if !is_pill {
        *POPUP_LOCKED.lock() = Some(frame);
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
    *POPUP_SLOT.lock() = None;
    *POPUP_LOCKED.lock() = None;
    unregister_escape(app);
    let _ = app.emit("popup-state", PopupState::Hidden);
}

/// Opens the quick panel and gives it focus (the user is about to type). `anchor` says where it
/// goes: under the tray icon, upper-centre for a shortcut, or beside a screen region (FR-024).
pub fn show_quick(app: &AppHandle, anchor: PanelAnchor) -> tauri::Result<()> {
    if focus_setup_if_open(app) {
        return Ok(());
    }
    let Some(window) = ensure_quick(app) else {
        return Ok(());
    };
    if records_capture_target(anchor) {
        // The app the user is in, so a screen capture started from this panel can hand the focus
        // back to it later (VM run 2026-11-24: it fell to the desktop). The platform never takes
        // one of Verso's own windows as the target.
        platform::services().remember_capture_target();
    }
    QUICK_SCREEN_MODE.store(false, Ordering::SeqCst);
    if QUICK_SCREEN_STALE.swap(false, Ordering::SeqCst) {
        // The page still shows the last capture (note, text, result): start from a clean panel.
        send_quick_screen(app, crate::core::quick_queue::QuickScreenEvent::reset());
    }
    *QUICK_ANCHOR.lock() = anchor;
    // A new panel shows once its page is ready, never as a blank frame.
    let now = QUICK.lock().request_show();
    if !now {
        return Ok(());
    }
    present_quick(&window, anchor)
}

fn anchor_point(anchor: PanelAnchor, cursor: Point) -> Point {
    match anchor {
        PanelAnchor::TrayIcon(rect) | PanelAnchor::Region(rect) => Point {
            x: rect.x + rect.width / 2.0,
            y: rect.y + rect.height / 2.0,
        },
        PanelAnchor::Shortcut => cursor,
    }
}

fn present_quick(window: &WebviewWindow, anchor: PanelAnchor) -> tauri::Result<()> {
    log::info!("quick panel: present");
    let services = platform::services();
    let work_area = services.monitor_work_area(anchor_point(anchor, services.cursor_position()));
    let size = Size {
        width: QUICK_WIDTH,
        height: QUICK_HEIGHT,
    };
    let position = place_panel(anchor, work_area, size, PLACEMENT_GAP);
    window.set_size(LogicalSize::new(size.width, size.height))?;
    window.set_position(LogicalPosition::new(position.x, position.y))?;
    window.show()?;
    window.set_focus()?;
    nudge_layout(window);
    // The focused panel handles Escape itself; only the popup needs the global registration (T056).
    Ok(())
}

/// Closes the panel. After a screen result the app that was in front before the capture gets the
/// focus back (FR-023).
pub fn hide_quick(app: &AppHandle) {
    hide_quick_with(app, true);
}

/// Closes the panel without handing the focus back (a blur, or a new capture about to start).
pub fn hide_quick_keep_focus(app: &AppHandle) {
    hide_quick_with(app, false);
}

fn hide_quick_with(app: &AppHandle, restore: bool) {
    QUICK.lock().hide();
    // Closed while a capture is read: its text must not reach the page (code review, FR-006).
    READING.lock().hide();
    let was_screen = QUICK_SCREEN_MODE.swap(false, Ordering::SeqCst);
    if let Some(window) = quick_window(app) {
        if window.is_visible().unwrap_or(false) {
            *QUICK_HIDDEN_AT.lock() = Some(std::time::Instant::now());
        }
        let _ = window.hide();
    }
    if was_screen && restore {
        platform::services().restore_capture_target();
    }
    // Escape belongs to the popup: hiding the panel must not unregister it.
}

pub fn toggle_quick(app: &AppHandle) {
    let pending = QUICK.lock().show_pending();
    if quick_is_visible(app) || pending {
        hide_quick(app);
    } else {
        let _ = show_quick(app, PanelAnchor::Shortcut);
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

pub const UPDATE_WIDTH: f64 = 600.0;
pub const UPDATE_MIN_HEIGHT: f64 = 220.0;
pub const UPDATE_MAX_HEIGHT: f64 = 560.0;

/// The height the update window takes for the content its page measured (spec 004 R11).
pub fn clamp_update_height(requested: f64) -> f64 {
    if requested.is_finite() {
        requested.clamp(UPDATE_MIN_HEIGHT, UPDATE_MAX_HEIGHT)
    } else {
        UPDATE_MIN_HEIGHT
    }
}

/// Fits the open update window to its content; the width stays, so does the top-left corner.
pub fn resize_update_window(app: &AppHandle, height: f64) {
    if let Some(window) = app.get_webview_window("update") {
        let _ = window.set_size(LogicalSize::new(UPDATE_WIDTH, clamp_update_height(height)));
    }
}

/// Opens (or focuses) the update dialog: 600 wide, as tall as its content (260 while checking, 460
/// for an offer at first), not resizable by the user, created hidden (R14).
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
    .inner_size(UPDATE_WIDTH, if mode == "offer" { 460.0 } else { 260.0 })
    .min_inner_size(UPDATE_WIDTH, UPDATE_MIN_HEIGHT)
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
                // The region picker takes Escape first: cancelling it leaves the popup alone.
                if crate::core::ocr::is_selecting() {
                    crate::core::ocr::cancel(app);
                    return;
                }
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
    // The overlays still need Escape while they are up (a popup hidden meanwhile must not drop it).
    if crate::core::ocr::is_selecting() {
        return;
    }
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
/// it lasts for the process so Settings › Permissions reads the same reason as the step (spec 004).
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

/// Opens the onboarding directly at the Accessibility step, saying why: missing at runtime, or lost
/// across an update (spec 010 FR-003).
pub fn show_permission_onboarding(app: &AppHandle, kind: PermissionKind, reason: PermissionReason) {
    set_permission_reason(kind, reason);
    *PERMISSION_STEP_KIND.lock() = kind;
    ONBOARDING_STEP.store(1, Ordering::SeqCst);
    show_onboarding(app);
}

/// The kind the permission step was opened for; reset to Accessibility when the window closes.
static PERMISSION_STEP_KIND: parking_lot::Mutex<PermissionKind> =
    parking_lot::Mutex::new(PermissionKind::Accessibility);

pub fn permission_step_kind() -> PermissionKind {
    *PERMISSION_STEP_KIND.lock()
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
                    *PERMISSION_STEP_KIND.lock() = PermissionKind::Accessibility;
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

// ---- spec 004: the region picker overlays (one window per monitor, created on demand) ----

const OVERLAY_PREFIX: &str = "capture-";

fn overlay_labels(app: &AppHandle) -> Vec<String> {
    app.webview_windows()
        .keys()
        .filter(|label| label.starts_with(OVERLAY_PREFIX))
        .cloned()
        .collect()
}

/// The monitors as logical frames in the global point space (`position / scale`, as Tauri reports
/// them on both platforms).
pub fn monitor_snapshots(app: &AppHandle) -> Vec<crate::core::ocr::MonitorInfo> {
    let Ok(monitors) = app.available_monitors() else {
        return Vec::new();
    };
    monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let scale = monitor.scale_factor();
            let position = monitor.position().to_logical::<f64>(scale);
            let size = monitor.size().to_logical::<f64>(scale);
            crate::core::ocr::MonitorInfo {
                index: index as u32,
                frame: Rect {
                    x: position.x,
                    y: position.y,
                    width: size.width,
                    height: size.height,
                },
                scale,
            }
        })
        .collect()
}

/// A hidden overlay kept ready for the primary monitor while the module is on (spec 004 R5
/// fallback, T054): the VM runs showed 245 ms to 1.7 s to create an overlay cold, over the 250 ms
/// budget. `ready` is set once its page has painted.
struct WarmOverlay {
    frame: Rect,
    ready: bool,
    created: std::time::Instant,
}

/// How long an unused warm overlay is kept. A kept-ready webview costs ~45 MB, so it lives only for
/// a burst of captures; an idle Verso goes back to its module-off footprint (spec 004 S20).
const WARM_LIFETIME: std::time::Duration = std::time::Duration::from_secs(120);

static WARM: parking_lot::Mutex<Option<(u32, WarmOverlay)>> = parking_lot::Mutex::new(None);

fn build_overlay(
    app: &AppHandle,
    monitor: &crate::core::ocr::MonitorInfo,
) -> Option<WebviewWindow> {
    let label = format!("{OVERLAY_PREFIX}{}", monitor.index);
    let builder = tauri::WebviewWindowBuilder::new(
        app,
        &label,
        tauri::WebviewUrl::App("capture.html".into()),
    )
    .title("Verso")
    .position(monitor.frame.x, monitor.frame.y)
    .inner_size(monitor.frame.width, monitor.frame.height)
    .visible(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .shadow(false)
    .accept_first_mouse(true);
    match builder.build() {
        Ok(window) => {
            info!("overlay {label} created");
            let styled = window.clone();
            let _ = app.run_on_main_thread(move || {
                platform::services().prepare_overlay(&styled);
            });
            Some(window)
        }
        Err(error) => {
            warn!("could not create overlay {label}: {error}");
            None
        }
    }
}

/// The monitor to keep a warm overlay for: the primary one (index in `monitor_snapshots`).
fn warm_target(app: &AppHandle) -> Option<crate::core::ocr::MonitorInfo> {
    let snapshots = monitor_snapshots(app);
    let primary = app.primary_monitor().ok().flatten();
    let index = primary
        .and_then(|primary| {
            let scale = primary.scale_factor();
            let position = primary.position().to_logical::<f64>(scale);
            snapshots.iter().position(|info| {
                (info.frame.x - position.x).abs() < 1.0 && (info.frame.y - position.y).abs() < 1.0
            })
        })
        .unwrap_or(0);
    snapshots.get(index).copied()
}

/// Creates the hidden overlay for the primary monitor if the module is on and none exists. Called
/// at start, when the module is switched on, and after each use; never while the picker is up.
pub fn warm_overlay(app: &AppHandle) {
    let enabled = app
        .state::<crate::commands::AppState>()
        .settings
        .get()
        .ocr
        .enabled;
    if !enabled
        || !platform::services().supports("ocr")
        || crate::core::ocr::is_selecting()
        || WARM.lock().is_some()
    {
        return;
    }
    let Some(monitor) = warm_target(app) else {
        return;
    };
    let created = std::time::Instant::now();
    // Claim the slot first so two callers cannot build the same window.
    *WARM.lock() = Some((
        monitor.index,
        WarmOverlay {
            frame: monitor.frame,
            ready: false,
            created,
        },
    ));
    if build_overlay(app, &monitor).is_none() {
        *WARM.lock() = None;
    } else {
        info!("warm overlay kept for monitor {}", monitor.index);
        // Expire it if nobody used it: the slot still holding this very overlay means it is idle.
        let handle = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(WARM_LIFETIME);
            let idle = WARM
                .lock()
                .as_ref()
                .is_some_and(|(_, overlay)| overlay.created == created);
            if idle && !crate::core::ocr::is_selecting() {
                info!("warm overlay expired unused");
                discard_warm(&handle);
            }
        });
    }
}

/// Drops the warm overlay (module switched off, or its frame no longer matches the monitor).
fn discard_warm(app: &AppHandle) {
    let taken = WARM.lock().take();
    if let Some((index, _)) = taken {
        if let Some(window) = app.get_webview_window(&format!("{OVERLAY_PREFIX}{index}")) {
            let _ = app.run_on_main_thread(move || {
                platform::services().close_overlay(&window);
            });
        }
    }
}

/// Builds the overlays for every monitor, reusing the warm one when it still fits. Each page calls
/// `overlay_ready` once painted and is shown then. Returns false when none could be created.
pub fn open_overlays(app: &AppHandle, monitors: &[crate::core::ocr::MonitorInfo]) -> bool {
    let mut created = 0;
    let mut show_now = Vec::new();
    for monitor in monitors {
        let reuse = {
            let warm = WARM.lock();
            match warm.as_ref() {
                Some((index, overlay))
                    if *index == monitor.index
                        && (overlay.frame.x - monitor.frame.x).abs() < 1.0
                        && (overlay.frame.y - monitor.frame.y).abs() < 1.0
                        && (overlay.frame.width - monitor.frame.width).abs() < 1.0
                        && (overlay.frame.height - monitor.frame.height).abs() < 1.0 =>
                {
                    Some(overlay.ready)
                }
                _ => None,
            }
        };
        match reuse {
            Some(ready) => {
                info!("overlay {} reused (warm, ready: {ready})", monitor.index);
                created += 1;
                // The warm slot is spent: the window now belongs to this session.
                WARM.lock().take();
                if ready {
                    show_now.push(monitor.index);
                }
            }
            None => {
                // A stale warm overlay for this monitor must go before a new one takes its label.
                let stale = WARM
                    .lock()
                    .as_ref()
                    .is_some_and(|(index, _)| *index == monitor.index);
                if stale {
                    discard_warm(app);
                }
                if build_overlay(app, monitor).is_some() {
                    created += 1;
                }
            }
        }
    }
    if created > 0 {
        register_escape(app);
        for index in show_now {
            show_overlay(app, index);
        }
        // Safety net: if a page never reports ready (a script error), show the overlays anyway.
        let late = app.clone();
        let indexes: Vec<u32> = monitors.iter().map(|monitor| monitor.index).collect();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(800));
            if crate::core::ocr::is_selecting() {
                for index in indexes {
                    show_overlay(&late, index);
                }
            }
        });
    }
    created > 0
}

/// An overlay page reports it has painted. While the picker is up it is shown at once; otherwise
/// it is the warm overlay and only its readiness is remembered.
pub fn overlay_page_ready(app: &AppHandle, monitor: u32) {
    if crate::core::ocr::is_selecting() {
        show_overlay(app, monitor);
        return;
    }
    if let Some((index, overlay)) = WARM.lock().as_mut() {
        if *index == monitor {
            overlay.ready = true;
            info!("warm overlay {monitor} is ready");
        }
    }
}

/// Puts an overlay on screen and tells its page to reset (hint, selection).
pub fn show_overlay(app: &AppHandle, monitor: u32) {
    info!("overlay {monitor} showing");
    let label = format!("{OVERLAY_PREFIX}{monitor}");
    let Some(window) = app.get_webview_window(&label) else {
        return;
    };
    let _ = app.emit_to(label, "overlay-activate", ());
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = &handle;
        platform::services().show_overlay(&window);
    });
}

/// The module was switched off: the warm overlay goes too (no idle cost while it is off).
pub fn release_warm_overlay(app: &AppHandle) {
    discard_warm(app);
}

/// Destroys every overlay and gives the foreground back.
pub fn close_overlays(app: &AppHandle) {
    close_overlays_with(app, true);
}

/// Closes the overlays; `restore_focus` hands the foreground back to the app that was in front
/// (not when a panel is about to take it).
pub fn close_overlays_with(app: &AppHandle, restore_focus: bool) {
    let labels = overlay_labels(app);
    if labels.is_empty() {
        return;
    }
    // Everything is destroyed below, a warm overlay included.
    WARM.lock().take();
    let (sender, receiver) = std::sync::mpsc::channel();
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        for label in labels {
            if let Some(window) = handle.get_webview_window(&label) {
                platform::services().close_overlay(&window);
            }
        }
        let _ = sender.send(());
    });
    // The capture that follows must not see the overlay: wait until it is gone.
    let _ = receiver.recv_timeout(std::time::Duration::from_millis(500));
    // Escape belongs to the popup again only when one is shown (it registers it itself).
    if let Some(popup) = popup_window(app) {
        if !popup.is_visible().unwrap_or(false) {
            unregister_escape(app);
        }
    } else {
        unregister_escape(app);
    }
    if restore_focus {
        platform::services().restore_capture_target();
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
