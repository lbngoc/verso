//! Non-activating window style on Windows (research P5):
//! `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST`, shown with `SW_SHOWNOACTIVATE`.

use log::debug;
use tauri::WebviewWindow;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE, SW_HIDE, SW_SHOWNOACTIVATE,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
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

/// First click inside activates the window so keyboard input works (research P5).
pub fn activate(window: &WebviewWindow) {
    let Some(hwnd) = hwnd_of(window) else {
        return;
    };
    unsafe {
        let _ = set_foreground_window_compat(hwnd);
    }
}

unsafe fn set_foreground_window_compat(hwnd: HWND) -> bool {
    windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd).as_bool()
}
