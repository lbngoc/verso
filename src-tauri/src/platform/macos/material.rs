//! Native material behind the popup/quick webviews (FR-020, popup-ui-contract § appearance matrix).
//!
//! macOS 26+: `NSGlassEffectView` (Liquid Glass). macOS 13–15: `NSVisualEffectView` with material
//! `.popover`, blending behind the window and the state forced `.active` (the panel is never key).
//! Reduce Transparency / Increase Contrast: no material view at all; the stylesheet paints an
//! opaque `windowBackgroundColor`-equivalent surface.

use log::debug;
use objc2::runtime::AnyObject;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSColor, NSGlassEffectView, NSGlassEffectViewStyle, NSView, NSVisualEffectBlendingMode,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow,
    NSWindowOrderingMode,
};
use objc2_foundation::NSRect;
use tauri::WebviewWindow;

use crate::platform::{Appearance, Material};

const AUTORESIZING: usize = 2 | 16; // NSViewWidthSizable | NSViewHeightSizable

fn remove_material_views(content: &NSView) {
    let subviews = content.subviews();
    for view in subviews.iter() {
        let any: &AnyObject = &view;
        let name = any.class().name().to_bytes();
        if name == b"NSVisualEffectView" || name == b"NSGlassEffectView" {
            view.removeFromSuperview();
        }
    }
}

pub fn apply(window: &WebviewWindow, appearance: &Appearance) {
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    let Some(mtm) = MainThreadMarker::new() else {
        debug!("material applied off the main thread; keeping the CSS fallback");
        return;
    };
    let ns_window: &NSWindow = unsafe { &*(pointer as *const NSWindow) };
    let Some(content) = ns_window.contentView() else {
        return;
    };

    remove_material_views(&content);

    match appearance.material {
        Material::Glass => {
            let view = NSGlassEffectView::new(mtm);
            view.setStyle(NSGlassEffectViewStyle::Regular);
            view.setCornerRadius(22.0);
            install(&content, &view, 22.0);
            debug!("installed NSGlassEffectView");
        }
        Material::Vibrancy => {
            let view = NSVisualEffectView::new(mtm);
            // .hudWindow reads better in dark mode; .popover is the system panel material.
            let material = if appearance.theme == crate::platform::UiTheme::Dark {
                NSVisualEffectMaterial::HUDWindow
            } else {
                NSVisualEffectMaterial::Popover
            };
            view.setMaterial(material);
            view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
            view.setState(NSVisualEffectState::Active);
            install(&content, &view, 16.0);
            debug!("installed NSVisualEffectView ({material:?}) forced active");
        }
        Material::Opaque | Material::Acrylic => {
            // Nothing behind the webview: tokens.css paints the opaque surface.
            let color = NSColor::windowBackgroundColor();
            ns_window.setBackgroundColor(Some(&color));
            ns_window.setOpaque(true);
            round_content(&content, 16.0);
            debug!("opaque surface (webview background)");
        }
    }
}

fn install(content: &NSView, view: &NSView, radius: f64) {
    let bounds: NSRect = content.bounds();
    view.setFrame(bounds);
    view.setAutoresizingMask(objc2_app_kit::NSAutoresizingMaskOptions(AUTORESIZING));
    content.addSubview_positioned_relativeTo(view, NSWindowOrderingMode::Below, None);
    // The window is borderless, so the rounded corner is the content view's layer.
    round_content(content, radius);
}

/// Clips the webview (and the material behind it) to the native continuous corner radius.
fn round_content(content: &NSView, radius: f64) {
    unsafe {
        let _: () = objc2::msg_send![content, setWantsLayer: true];
        let layer: *mut AnyObject = objc2::msg_send![content, layer];
        if !layer.is_null() {
            let _: () = objc2::msg_send![layer, setCornerRadius: radius];
            let _: () = objc2::msg_send![layer, setMasksToBounds: true];
        }
    }
}
