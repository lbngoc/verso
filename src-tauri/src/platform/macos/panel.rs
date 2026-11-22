//! Non-activating panel setup (research P5): borderless NSPanel behaviour on the popup and quick
//! windows — joins all Spaces and full-screen spaces, floats above normal windows, and is ordered
//! front without activating the app.

use log::debug;
use objc2_app_kit::{
    NSFloatingWindowLevel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use tauri::{Manager, WebviewWindow};
use tauri_nspanel::{ManagerExt, WebviewWindowExt};

mod popup_panel {
    use tauri_nspanel::tauri_panel;

    tauri_panel! {
        // The popup: a floating, non-activating panel that can still become key, so `c`, `s`, `-`,
        // `=`, `0` and Esc reach it while the app the user is in stays active (spec 009 FR-032).
        panel!(PopupPanel {
            config: {
                can_become_key_window: true,
                can_become_main_window: false,
                become_key_if_only_needed: false,
                is_floating_panel: true
            }
        })
    }
}
use popup_panel::PopupPanel;

mod overlay_panel {
    use tauri_nspanel::tauri_panel;

    tauri_panel! {
        // The region picker (spec 004): covers a monitor above the menu bar, takes the keyboard
        // and the mouse without activating Verso, so the app the user is in keeps its state.
        panel!(OverlayPanel {
            config: {
                can_become_key_window: true,
                can_become_main_window: false,
                become_key_if_only_needed: false,
                is_floating_panel: true
            }
        })
    }
}
use overlay_panel::OverlayPanel;

/// `NSScreenSaverWindowLevel`: above the menu bar and the Dock.
const OVERLAY_LEVEL: i64 = 1000;

/// `NSWindowStyleMaskNonactivatingPanel`.
const NONACTIVATING_PANEL: usize = 1 << 7;

pub fn prepare(window: &WebviewWindow) {
    if window.label() == "popup" {
        match window.to_panel::<PopupPanel>() {
            Ok(panel) => {
                let _ = panel.add_style_mask(objc2_app_kit::NSWindowStyleMask::NonactivatingPanel);
                panel.set_collection_behavior(NSWindowCollectionBehavior(
                    NSWindowCollectionBehavior::CanJoinAllSpaces.0
                        | NSWindowCollectionBehavior::Stationary.0
                        | NSWindowCollectionBehavior::FullScreenAuxiliary.0,
                ));
                panel.set_level(NSFloatingWindowLevel as i64);
                panel.set_hides_on_deactivate(false);
                debug!("popup converted to a non-activating NSPanel");
                return;
            }
            Err(error) => debug!("popup panel conversion failed, using a plain window: {error}"),
        }
    }
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
    if let Ok(panel) = window.app_handle().get_webview_panel(window.label()) {
        panel.order_front_regardless();
        return;
    }
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
    if let Ok(panel) = window.app_handle().get_webview_panel(window.label()) {
        panel.hide();
        return;
    }
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
    if let Ok(panel) = window.app_handle().get_webview_panel(window.label()) {
        panel.make_key_window();
        return;
    }
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    let ns_window: &NSWindow = unsafe { &*(pointer as *const NSWindow) };
    ns_window.makeKeyWindow();
}

/// Turns an overlay window into a non-activating panel above the menu bar on every Space.
pub fn prepare_overlay(window: &WebviewWindow) {
    match window.to_panel::<OverlayPanel>() {
        Ok(panel) => {
            let _ = panel.add_style_mask(NSWindowStyleMask::NonactivatingPanel);
            panel.set_collection_behavior(NSWindowCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces.0
                    | NSWindowCollectionBehavior::Stationary.0
                    | NSWindowCollectionBehavior::FullScreenAuxiliary.0,
            ));
            panel.set_level(OVERLAY_LEVEL);
            panel.set_hides_on_deactivate(false);
            panel.set_has_shadow(false);
            debug!("overlay converted to a panel");
        }
        Err(error) => debug!("overlay panel conversion failed, using a plain window: {error}"),
    }
}

/// Shows the overlay and makes it key (for Escape) without activating the app.
pub fn show_overlay(window: &WebviewWindow) {
    if let Ok(panel) = window.app_handle().get_webview_panel(window.label()) {
        panel.order_front_regardless();
        panel.make_key_window();
        return;
    }
    let _ = window.show();
}

/// A panel must become a plain window again before Tauri destroys it.
pub fn close_overlay(window: &WebviewWindow) {
    if let Ok(panel) = window.app_handle().get_webview_panel(window.label()) {
        panel.hide();
        if let Some(plain) = panel.to_window() {
            let _ = plain.destroy();
            return;
        }
    }
    let _ = window.destroy();
}
