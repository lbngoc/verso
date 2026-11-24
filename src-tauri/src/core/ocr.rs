//! Screen translate (spec 004): the capture session and the flow that turns a dragged region into
//! a translation. The session rules and the coordinate maths are pure and tested here; the OS
//! work sits behind `PlatformServices` and the windows behind `window_manager`.
//!
//! Logs carry timings, counts and language codes only: never recognised text or image data
//! (FR-006, `tests/no_text_logging.rs`).

use std::time::Instant;

use log::{debug, info, warn};
use parking_lot::Mutex;
use tauri::{AppHandle, Manager};

use crate::commands::AppState;
use crate::core::ocr_text;
use crate::core::quick_queue::QuickScreenEvent;
use crate::platform::{self, CaptureError, CaptureRegion, PermissionKind, PermissionStatus, Rect};
use crate::window_manager;

/// A drag smaller than this many points on a side is a click, which cancels.
pub const MIN_SELECTION: f64 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    Idle,
    Selecting,
    Capturing,
    Recognising,
    Translating,
}

/// One connected monitor at the moment the overlays opened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorInfo {
    pub index: u32,
    /// Global logical points.
    pub frame: Rect,
    pub scale: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartResult {
    Opened,
    Ignored,
    Permission,
}

/// What a finished drag asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Release {
    Cancelled,
    Capture(CaptureRegion),
}

/// What happens after the capture attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishOutcome {
    Continue,
    /// Screen Recording was revoked while Verso runs: no recognition, no popup; the permission
    /// step opens instead (spec 004 US2 AS2).
    Permission,
    Failed,
}

/// The decision of `start`, free of OS calls so it can be tested (T026, T044).
pub fn start_decision(
    module_on: bool,
    available: bool,
    state: CaptureState,
    permission: PermissionStatus,
) -> StartResult {
    if !module_on || !available || state != CaptureState::Idle {
        return StartResult::Ignored;
    }
    if permission != PermissionStatus::Granted {
        return StartResult::Permission;
    }
    StartResult::Opened
}

/// Maps the capture result to the next step.
pub fn classify_capture<T>(result: &Result<T, CaptureError>) -> FinishOutcome {
    match result {
        Ok(_) => FinishOutcome::Continue,
        Err(CaptureError::PermissionMissing) => FinishOutcome::Permission,
        Err(CaptureError::Failed(_)) => FinishOutcome::Failed,
    }
}

/// A rect relative to a monitor's frame, as a global logical rect.
pub fn to_global(frame: Rect, relative: Rect) -> Rect {
    Rect {
        x: frame.x + relative.x,
        y: frame.y + relative.y,
        width: relative.width,
        height: relative.height,
    }
}

/// A global logical rect in physical pixels of the monitor it lies on.
pub fn to_physical(global: Rect, scale: f64) -> Rect {
    Rect {
        x: (global.x * scale).round(),
        y: (global.y * scale).round(),
        width: (global.width * scale).round(),
        height: (global.height * scale).round(),
    }
}

#[derive(Debug)]
pub struct CaptureSession {
    state: CaptureState,
    monitors: Vec<MonitorInfo>,
}

impl CaptureSession {
    pub const fn new() -> Self {
        Self {
            state: CaptureState::Idle,
            monitors: Vec::new(),
        }
    }

    pub fn state(&self) -> CaptureState {
        self.state
    }

    /// `Idle → Selecting`; ignored in any other state.
    pub fn try_start(&mut self, monitors: Vec<MonitorInfo>) -> StartResult {
        if self.state != CaptureState::Idle || monitors.is_empty() {
            return StartResult::Ignored;
        }
        self.monitors = monitors;
        self.state = CaptureState::Selecting;
        StartResult::Opened
    }

    /// `Selecting → Idle`.
    pub fn cancel(&mut self) -> bool {
        if self.state == CaptureState::Selecting {
            self.state = CaptureState::Idle;
            self.monitors.clear();
            true
        } else {
            false
        }
    }

    /// A drag ended on `monitor`: too small cancels, otherwise `Selecting → Capturing` with the
    /// region in global points and physical pixels.
    pub fn release(&mut self, monitor: u32, rect: Rect) -> Release {
        if self.state != CaptureState::Selecting {
            return Release::Cancelled;
        }
        let info = self
            .monitors
            .iter()
            .find(|info| info.index == monitor)
            .copied();
        let Some(info) = info else {
            self.cancel();
            return Release::Cancelled;
        };
        if rect.width < MIN_SELECTION || rect.height < MIN_SELECTION {
            self.cancel();
            return Release::Cancelled;
        }
        let logical = to_global(info.frame, rect);
        self.state = CaptureState::Capturing;
        Release::Capture(CaptureRegion {
            logical,
            physical: to_physical(logical, info.scale),
            scale: info.scale,
        })
    }

    /// Moves forward one step of the flow (`Capturing → Recognising → Translating`).
    pub fn advance(&mut self) {
        self.state = match self.state {
            CaptureState::Capturing => CaptureState::Recognising,
            CaptureState::Recognising => CaptureState::Translating,
            other => other,
        };
    }

    /// Back to `Idle` from anywhere.
    pub fn finish(&mut self) {
        self.state = CaptureState::Idle;
        self.monitors.clear();
    }
}

impl Default for CaptureSession {
    fn default() -> Self {
        Self::new()
    }
}

static SESSION: Mutex<CaptureSession> = Mutex::new(CaptureSession::new());

/// True while the overlays are up (the Escape shortcut cancels them).
pub fn is_selecting() -> bool {
    SESSION.lock().state() == CaptureState::Selecting
}

/// Opens the overlays on every monitor; the shortcut, the tray item and the popup's "try another
/// region" all come here.
pub fn start(app: &AppHandle) -> StartResult {
    info!("screen translate requested");
    let state = app.state::<AppState>();
    let module_on = state.settings.get().ocr.enabled;
    let services = platform::services();
    let available = services.supports("ocr");
    let permission = services.permission_status(PermissionKind::ScreenRecording);
    let decision = start_decision(module_on, available, SESSION.lock().state(), permission);
    match decision {
        StartResult::Ignored => {
            debug!("screen translate ignored (module off, unavailable or already running)");
            return StartResult::Ignored;
        }
        StartResult::Permission => {
            info!("screen translate needs the Screen Recording permission");
            open_permission_step(app);
            return StartResult::Permission;
        }
        StartResult::Opened => {}
    }

    let started = Instant::now();
    // A popup or panel from the previous capture is closed first so it cannot sit under the
    // overlay. A panel showing a screen result keeps the app that was in front before it as the
    // focus target, so closing it later still returns there (FR-023).
    window_manager::hide_popup(app);
    // The target is taken before the panel hides (it may still be in front a moment after), and
    // the platform never takes one of Verso's own windows as the target.
    let was_screen_result = window_manager::quick_in_screen_mode();
    if !was_screen_result {
        services.remember_capture_target();
    }
    window_manager::hide_quick_keep_focus(app);
    let monitors = window_manager::monitor_snapshots(app);
    if SESSION.lock().try_start(monitors.clone()) != StartResult::Opened {
        return StartResult::Ignored;
    }
    if !window_manager::open_overlays(app, &monitors) {
        warn!("could not open the region overlays");
        SESSION.lock().finish();
        window_manager::close_overlays(app);
        return StartResult::Ignored;
    }
    info!("overlays created in {} ms", started.elapsed().as_millis());
    // The panel is built hidden while the user drags, so the result shows without a cold start.
    let _ = window_manager::ensure_quick(app);
    StartResult::Opened
}

/// Escape, right-click or a click without a drag.
pub fn cancel(app: &AppHandle) {
    let was_selecting = SESSION.lock().cancel();
    info!("screen translate cancelled (was selecting: {was_selecting})");
    window_manager::close_overlays(app);
    if was_selecting {
        debug!("screen translate cancelled");
    }
    rewarm(app);
}

/// Keeps the next start fast: a hidden overlay is rebuilt a moment after each use.
fn rewarm(app: &AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        window_manager::warm_overlay(&handle);
    });
}

/// The user released the drag: capture, read, and hand the text to the quick panel, which shows
/// it next to the region, editable, and translates it (spec 004 Revision 2, FR-004, FR-018).
pub async fn finish(app: AppHandle, monitor: u32, rect: Rect) {
    finish_flow(app.clone(), monitor, rect).await;
    rewarm(&app);
}

async fn finish_flow(app: AppHandle, monitor: u32, rect: Rect) {
    let region = match SESSION.lock().release(monitor, rect) {
        Release::Cancelled => {
            window_manager::close_overlays(&app);
            return;
        }
        Release::Capture(region) => region,
    };
    // The overlays go first so the shot shows the screen as it is at the moment of release. The
    // focus is not handed back: the panel takes it next, and a permission step does not need it.
    window_manager::close_overlays_with(&app, false);
    let services = platform::services();
    if !services.overlay_excluded_from_capture() {
        // The compositor needs a frame to draw the screen without the overlay.
        tokio::time::sleep(std::time::Duration::from_millis(70)).await;
    }

    let started = Instant::now();
    let captured = tokio::task::spawn_blocking(move || platform::services().capture_region(region))
        .await
        .unwrap_or_else(|error| {
            Err(CaptureError::Failed(format!(
                "capture task failed: {error}"
            )))
        });
    let capture_ms = started.elapsed().as_millis();
    let captured_at = Instant::now();

    match classify_capture(&captured) {
        FinishOutcome::Permission => {
            // Granted when it was checked, revoked since: never show a black capture.
            warn!("screen capture refused: the Screen Recording permission is missing");
            SESSION.lock().finish();
            open_permission_step(&app);
            return;
        }
        FinishOutcome::Failed => {
            warn!("screen capture failed");
            // The panel shows in its reading state first, then the failure.
            show_panel(&app, region.logical, captured_at);
            window_manager::send_quick_screen(&app, QuickScreenEvent::read_failed());
            SESSION.lock().finish();
            return;
        }
        FinishOutcome::Continue => {}
    }
    let Ok(image) = captured else {
        SESSION.lock().finish();
        return;
    };
    SESSION.lock().advance();

    // Shown only after the capture: on Windows the panel would otherwise be in the picture.
    show_panel(&app, region.logical, captured_at);

    let settings = app.state::<AppState>().settings.get();
    let language = recognition_language(&settings.ocr.language);
    let started = Instant::now();
    let recognised = tokio::task::spawn_blocking(move || {
        let outcome = platform::services().recognise(image, &language);
        // `image` was moved into `recognise` and is dropped there: nothing keeps the pixels.
        outcome
    })
    .await
    .unwrap_or_else(|error| {
        Err(platform::OcrError(format!(
            "recognition task failed: {error}"
        )))
    });
    let ocr_ms = started.elapsed().as_millis();

    let lines = match recognised {
        Ok(lines) => lines,
        Err(error) => {
            warn!("text recognition failed: {}", error.0);
            window_manager::send_quick_screen(&app, QuickScreenEvent::read_failed());
            SESSION.lock().finish();
            return;
        }
    };
    let text = ocr_text::join_lines(&lines);
    let line_count = lines.len();
    drop(lines);
    SESSION.lock().advance();

    if text.trim().is_empty() {
        // Nothing readable: no engine request is made (FR-005).
        info!("capture {capture_ms} ms, ocr {ocr_ms} ms, lines 0, chars 0: no text in the region");
        window_manager::send_quick_screen(&app, QuickScreenEvent::no_text());
        SESSION.lock().finish();
        return;
    }

    let char_count = text.chars().count();
    info!("capture {capture_ms} ms, ocr {ocr_ms} ms, lines {line_count}, chars {char_count}");
    // The panel translates it (origin `screen`) and the user may correct the text first.
    window_manager::send_quick_screen(&app, QuickScreenEvent::text(text));
    SESSION.lock().finish();
}

/// Resets the panel to "Reading the text…", places it by the screen-result rule and shows it;
/// logs how long after the capture it became visible (never any text).
fn show_panel(app: &AppHandle, region: Rect, captured_at: Instant) {
    window_manager::prepare_quick_screen(app, region);
    info!("panel shown in {} ms", captured_at.elapsed().as_millis());
}

/// An unknown code falls back to "auto" (logged, the setting is kept), data-model OcrSettings.
fn recognition_language(configured: &str) -> String {
    let availability = platform::services().ocr_availability();
    resolve_language(configured, &availability.languages)
}

pub fn resolve_language(configured: &str, known: &[platform::OcrLanguage]) -> String {
    if configured == "auto" || known.is_empty() {
        return "auto".into();
    }
    let wanted = configured.to_ascii_lowercase();
    let matches = known.iter().any(|language| {
        let code = language.code.to_ascii_lowercase();
        code == wanted
            || code.split('-').next() == wanted.split('-').next() && !wanted.contains('-')
    });
    if matches {
        configured.to_string()
    } else {
        info!("recognition language {configured} is not available; using auto");
        "auto".into()
    }
}

fn open_permission_step(app: &AppHandle) {
    window_manager::hide_popup(app);
    window_manager::show_permission_onboarding(
        app,
        PermissionKind::ScreenRecording,
        window_manager::permission_reason(PermissionKind::ScreenRecording),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(index: u32, x: f64, y: f64, scale: f64) -> MonitorInfo {
        MonitorInfo {
            index,
            frame: Rect {
                x,
                y,
                width: 1440.0,
                height: 900.0,
            },
            scale,
        }
    }

    fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn the_session_walks_idle_to_idle() {
        let mut session = CaptureSession::new();
        assert_eq!(session.state(), CaptureState::Idle);
        assert_eq!(
            session.try_start(vec![monitor(0, 0.0, 0.0, 2.0)]),
            StartResult::Opened
        );
        assert_eq!(session.state(), CaptureState::Selecting);
        let release = session.release(0, rect(10.0, 20.0, 100.0, 50.0));
        assert!(matches!(release, Release::Capture(_)));
        assert_eq!(session.state(), CaptureState::Capturing);
        session.advance();
        assert_eq!(session.state(), CaptureState::Recognising);
        session.advance();
        assert_eq!(session.state(), CaptureState::Translating);
        session.finish();
        assert_eq!(session.state(), CaptureState::Idle);
    }

    #[test]
    fn cancel_returns_to_idle_only_from_selecting() {
        let mut session = CaptureSession::new();
        assert!(!session.cancel());
        session.try_start(vec![monitor(0, 0.0, 0.0, 1.0)]);
        assert!(session.cancel());
        assert_eq!(session.state(), CaptureState::Idle);
    }

    #[test]
    fn a_second_start_is_ignored_unless_idle() {
        let mut session = CaptureSession::new();
        session.try_start(vec![monitor(0, 0.0, 0.0, 1.0)]);
        assert_eq!(
            session.try_start(vec![monitor(0, 0.0, 0.0, 1.0)]),
            StartResult::Ignored
        );
        for state in [
            CaptureState::Selecting,
            CaptureState::Capturing,
            CaptureState::Recognising,
            CaptureState::Translating,
        ] {
            assert_eq!(
                start_decision(true, true, state, PermissionStatus::Granted),
                StartResult::Ignored,
                "{state:?}"
            );
        }
    }

    #[test]
    fn a_selection_under_four_points_is_a_cancel() {
        for (width, height) in [(3.0, 50.0), (50.0, 3.0), (0.0, 0.0)] {
            let mut session = CaptureSession::new();
            session.try_start(vec![monitor(0, 0.0, 0.0, 1.0)]);
            assert_eq!(
                session.release(0, rect(5.0, 5.0, width, height)),
                Release::Cancelled
            );
            assert_eq!(session.state(), CaptureState::Idle);
        }
        let mut session = CaptureSession::new();
        session.try_start(vec![monitor(0, 0.0, 0.0, 1.0)]);
        assert!(matches!(
            session.release(0, rect(5.0, 5.0, 4.0, 4.0)),
            Release::Capture(_)
        ));
    }

    #[test]
    fn a_missing_permission_stops_the_flow_before_any_recognition() {
        let denied: Result<(), CaptureError> = Err(CaptureError::PermissionMissing);
        assert_eq!(classify_capture(&denied), FinishOutcome::Permission);
        let failed: Result<(), CaptureError> = Err(CaptureError::Failed("x".into()));
        assert_eq!(classify_capture(&failed), FinishOutcome::Failed);
        assert_eq!(classify_capture(&Ok(())), FinishOutcome::Continue);
    }

    #[test]
    fn start_asks_for_the_permission_only_when_it_is_missing() {
        let decide = |permission| start_decision(true, true, CaptureState::Idle, permission);
        assert_eq!(decide(PermissionStatus::Granted), StartResult::Opened);
        assert_eq!(decide(PermissionStatus::Denied), StartResult::Permission);
        assert_eq!(
            decide(PermissionStatus::NotDetermined),
            StartResult::Permission
        );
        // Module off or unavailable: nothing happens, whatever the permission.
        assert_eq!(
            start_decision(false, true, CaptureState::Idle, PermissionStatus::Denied),
            StartResult::Ignored
        );
        assert_eq!(
            start_decision(true, false, CaptureState::Idle, PermissionStatus::Granted),
            StartResult::Ignored
        );
    }

    #[test]
    fn regions_convert_to_global_points_and_physical_pixels() {
        let frame = rect(-1440.0, 100.0, 1440.0, 900.0); // a monitor left of the primary
        let global = to_global(frame, rect(10.0, 20.0, 200.0, 80.0));
        assert_eq!(global, rect(-1430.0, 120.0, 200.0, 80.0));
        for (scale, expected) in [
            (1.0, rect(-1430.0, 120.0, 200.0, 80.0)),
            (1.5, rect(-2145.0, 180.0, 300.0, 120.0)),
            (2.0, rect(-2860.0, 240.0, 400.0, 160.0)),
        ] {
            assert_eq!(to_physical(global, scale), expected, "scale {scale}");
        }
    }

    #[test]
    fn release_reports_both_coordinate_systems() {
        let mut session = CaptureSession::new();
        session.try_start(vec![
            monitor(0, 0.0, 0.0, 1.0),
            monitor(1, 1440.0, 0.0, 2.0),
        ]);
        let Release::Capture(region) = session.release(1, rect(10.0, 20.0, 100.0, 50.0)) else {
            panic!("expected a capture");
        };
        assert_eq!(region.logical, rect(1450.0, 20.0, 100.0, 50.0));
        assert_eq!(region.physical, rect(2900.0, 40.0, 200.0, 100.0));
        assert_eq!(region.scale, 2.0);
    }

    #[test]
    fn an_unknown_language_falls_back_to_auto() {
        let known = vec![
            platform::OcrLanguage {
                code: "en-US".into(),
                name: "English".into(),
            },
            platform::OcrLanguage {
                code: "vi-VN".into(),
                name: "Vietnamese".into(),
            },
        ];
        assert_eq!(resolve_language("auto", &known), "auto");
        assert_eq!(resolve_language("en-US", &known), "en-US");
        assert_eq!(resolve_language("vi", &known), "vi");
        assert_eq!(resolve_language("xx-YY", &known), "auto");
        assert_eq!(resolve_language("fr", &[]), "auto");
    }
}
