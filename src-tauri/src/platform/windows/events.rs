//! Foreground change monitor: closes an unpinned popup when another window takes the foreground
//! (spec 009, R5). `EVENT_SYSTEM_FOREGROUND` is delivered out of context to the installing thread,
//! so the hook lives on the main thread, where Tauri pumps messages.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::OnceLock;

use log::debug;
use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{
    SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK, WINEVENTPROC,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
};

use crate::window_manager;

static APP: OnceLock<AppHandle> = OnceLock::new();
static HOOK: AtomicIsize = AtomicIsize::new(0);
static POPUP: AtomicIsize = AtomicIsize::new(0);
static INSTALLING: AtomicBool = AtomicBool::new(false);

pub fn start(app: AppHandle) {
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = APP.set(handle.clone());
        if let Some(window) = handle.get_webview_window("popup") {
            if let Ok(hwnd) = window.hwnd() {
                POPUP.store(hwnd.0 as isize, Ordering::SeqCst);
            }
        }
        if HOOK.load(Ordering::SeqCst) == 0 {
            let hook = unsafe {
                SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    None,
                    Some(on_foreground),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                )
            };
            HOOK.store(hook.0 as isize, Ordering::SeqCst);
            debug!("installed the foreground hook");
        }
        INSTALLING.store(false, Ordering::SeqCst);
    });
}

pub fn stop() {
    let raw = HOOK.swap(0, Ordering::SeqCst);
    if raw != 0 {
        unsafe { UnhookWinEvent(HWINEVENTHOOK(raw as *mut core::ffi::c_void)) };
        debug!("removed the foreground hook");
    }
}

unsafe extern "system" fn on_foreground(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    if window_manager::POPUP_PINNED.load(Ordering::SeqCst) {
        return;
    }
    let popup = POPUP.load(Ordering::SeqCst);
    if popup == 0 {
        return;
    }
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.0 as isize == popup {
        return;
    }
    if let Some(app) = APP.get() {
        window_manager::hide_popup(app);
    }
}
