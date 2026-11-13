//! Global mouse monitor: closes an unpinned popup when the user clicks outside it (spec 009, R5).
//!
//! Mouse monitors (`NSEvent.addGlobalMonitorForEvents…`) see clicks in other apps without the
//! Accessibility grant; the monitor exists only while the popup is visible.

use std::sync::atomic::Ordering;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSEvent, NSEventMask, NSWindow};
use parking_lot::Mutex;
use tauri::{AppHandle, Manager};

use crate::window_manager;

static MONITOR: Mutex<Option<MonitorHandle>> = Mutex::new(None);

/// The monitor token is an Objective-C object owned by the main run loop; the mutex only guards the
/// slot between the main thread's start/stop calls.
struct MonitorHandle(Retained<AnyObject>);

unsafe impl Send for MonitorHandle {}
unsafe impl Sync for MonitorHandle {}

pub fn start(app: AppHandle) {
    // NSEvent monitors belong to the main run loop; show_popup runs on a worker thread.
    if objc2::MainThreadMarker::new().is_none() {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || start_on_main(handle));
        return;
    }
    start_on_main(app);
}

fn start_on_main(app: AppHandle) {
    let mut slot = MONITOR.lock();
    if slot.is_some() {
        return;
    }
    let handle = app.clone();
    let block = block2::RcBlock::new(move |_event: std::ptr::NonNull<NSEvent>| {
        if window_manager::POPUP_PINNED.load(Ordering::SeqCst) {
            return;
        }
        let Some(window) = handle.get_webview_window("popup") else {
            return;
        };
        if !window.is_visible().unwrap_or(false) {
            return;
        }
        let Ok(pointer) = window.ns_window() else {
            return;
        };
        let ns_window: &NSWindow = unsafe { &*(pointer as *const NSWindow) };
        let frame = ns_window.frame();
        let point = NSEvent::mouseLocation();
        let inside = point.x >= frame.origin.x
            && point.x <= frame.origin.x + frame.size.width
            && point.y >= frame.origin.y
            && point.y <= frame.origin.y + frame.size.height;
        if inside {
            return;
        }
        window_manager::hide_popup(&handle);
    });
    let mask = NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown;
    let monitor = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &block);
    *slot = monitor.map(MonitorHandle);
}

pub fn stop() {
    if let Some(monitor) = MONITOR.lock().take() {
        unsafe { NSEvent::removeMonitor(&monitor.0) };
    }
}

/// Whether a monitor is installed (tests and diagnostics).
pub fn is_running() -> bool {
    MONITOR.lock().is_some()
}
