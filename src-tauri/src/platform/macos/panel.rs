//! Non-activating panel setup (research P5): borderless NSPanel behaviour on the popup and quick
//! windows — joins all Spaces and full-screen spaces, floats above normal windows, and is ordered
//! front without activating the app.

use log::debug;
use objc2_app_kit::{
    NSFloatingWindowLevel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use tauri::WebviewWindow;

/// `NSWindowStyleMaskNonactivatingPanel`.
const NONACTIVATING_PANEL: usize = 1 << 7;

pub fn prepare(window: &WebviewWindow) {
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    let ns_window: &NSWindow = unsafe { &*(pointer as *const NSWindow) };

    let style = ns_window.styleMask().0 | NONACTIVATING_PANEL;
    ns_window.setStyleMask(NSWindowStyleMask(style));

    let behavior = NSWindowCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces.0
            | NSWindowCollectionBehavior::Stationary.0
            | NSWindowCollectionBehavior::FullScreenAuxiliary.0,
    );
    ns_window.setCollectionBehavior(behavior);
    ns_window.setLevel(NSFloatingWindowLevel);
    ns_window.setHidesOnDeactivate(false);
    debug!("popup configured as a non-activating floating panel");
}

/// Shows the window without taking focus from the app the user is in (FR-002).
pub fn show_without_activation(window: &WebviewWindow) {
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    let ns_window: &NSWindow = unsafe { &*(pointer as *const NSWindow) };
    unsafe {
        // `orderFrontRegardless` shows the window without making it key.
        let _: () = objc2::msg_send![ns_window, orderFrontRegardless];
    }
}

/// Hides the panel natively (Tauri does not know it was shown outside its own `show`).
pub fn hide(window: &WebviewWindow) {
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    let ns_window: &NSWindow = unsafe { &*(pointer as *const NSWindow) };
    unsafe {
        let _: () =
            objc2::msg_send![ns_window, orderOut: std::ptr::null::<objc2::runtime::AnyObject>()];
    }
}

/// The panel becomes key only when the user clicks or types into it.
pub fn focus_on_interaction(window: &WebviewWindow) {
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    let ns_window: &NSWindow = unsafe { &*(pointer as *const NSWindow) };
    ns_window.makeKeyWindow();
}
