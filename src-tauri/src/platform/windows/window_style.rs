//! Non-activating window style on Windows (research P5):
//! `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST`, shown with `SW_SHOWNOACTIVATE`.

use log::debug;
use tauri::WebviewWindow;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE, SW_HIDE, SW_SHOW,
    SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

fn hwnd_of(window: &WebviewWindow) -> Option<HWND> {
    window.hwnd().ok().map(|hwnd| HWND(hwnd.0))
}

pub fn prepare(window: &WebviewWindow) {
    let Some(hwnd) = hwnd_of(window) else {
        return;
    };
    let current = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let wanted = current
        | WS_EX_NOACTIVATE.0 as isize
        | WS_EX_TOOLWINDOW.0 as isize
        | WS_EX_TOPMOST.0 as isize;
    unsafe {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, wanted);
    }
    debug!("popup configured as a non-activating tool window");
}

pub fn show_without_activation(window: &WebviewWindow) {
    let Some(hwnd) = hwnd_of(window) else {
        return;
    };
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

/// Hides the popup natively: its show bypasses Tauri's visibility tracking, so `window.hide()`
/// alone can be a no-op.
pub fn hide(window: &WebviewWindow) {
    let Some(hwnd) = hwnd_of(window) else {
        return;
    };
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

/// Whether the overlay could be kept out of screen captures (`WDA_EXCLUDEFROMCAPTURE`).
static OVERLAY_EXCLUDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn overlay_excluded() -> bool {
    OVERLAY_EXCLUDED.load(std::sync::atomic::Ordering::SeqCst)
}

/// The region picker: a top-most tool window with no taskbar button, kept out of captures
/// (spec 004 R3). When the OS refuses the exclusion, the flow waits for the overlay to go away.
pub fn prepare_overlay(window: &WebviewWindow) {
    let Some(hwnd) = hwnd_of(window) else {
        return;
    };
    let current = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let wanted = current | WS_EX_TOOLWINDOW.0 as isize | WS_EX_TOPMOST.0 as isize;
    unsafe {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, wanted);
        match windows::Win32::UI::WindowsAndMessaging::SetWindowDisplayAffinity(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::WDA_EXCLUDEFROMCAPTURE,
        ) {
            Ok(()) => OVERLAY_EXCLUDED.store(true, std::sync::atomic::Ordering::SeqCst),
            Err(error) => {
                log::info!("overlay cannot be excluded from capture: {error}");
                OVERLAY_EXCLUDED.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }
}

/// Puts the overlay on screen and gives it the keyboard so Escape works.
pub fn show_overlay(window: &WebviewWindow) {
    let Some(hwnd) = hwnd_of(window) else {
        return;
    };
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = set_foreground_window_compat(hwnd);
    }
}

/// First click inside activates the window so keyboard input works (research P5).
pub fn activate(window: &WebviewWindow) {
    let Some(hwnd) = hwnd_of(window) else {
        return;
    };
    unsafe {
        let _ = set_foreground_window_compat(hwnd);
    }
}

/// Makes the popup the foreground window on hover/click so its page receives the keys (R4).
pub fn focus_popup(window: &WebviewWindow) {
    let Some(hwnd) = hwnd_of(window) else {
        return;
    };
    unsafe {
        if !set_foreground_window_compat(hwnd) {
            log::warn!("SetForegroundWindow refused for the popup; hover or click will focus it");
        }
    }
}

/// Gives the foreground back to the window the selection came from when the popup hides.
pub fn restore_capture_target() {
    super::selection::restore_capture_target();
}

unsafe fn set_foreground_window_compat(hwnd: HWND) -> bool {
    windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd).as_bool()
}
