//! Native material behind the popup/quick webviews (FR-020, popup-ui-contract § appearance matrix).
//!
//! macOS 26+: `NSGlassEffectView` (Liquid Glass). macOS 13–15: `NSVisualEffectView` with material
//! `.popover`, blending behind the window and the state forced `.active` (the panel is never key).
//! Reduce Transparency / Increase Contrast: no material view at all; the stylesheet paints an
//! opaque `windowBackgroundColor`-equivalent surface.

use log::{debug, warn};
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

/// Corner radii shared with `--radius-large` in `src/styles/macos.css` (glass 22, others 16).
const GLASS_RADIUS: f64 = 22.0;
const PANEL_RADIUS: f64 = 16.0;

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
        warn!("material applied off the main thread; keeping the CSS fallback");
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
            view.setCornerRadius(GLASS_RADIUS);
            follow_theme(&view, appearance.theme);
            // Liquid Glass adapts its shade to what lies behind it, but the page's text follows
            // Verso's theme: light text over glass that went light (or dark over dark) was nearly
            // invisible (T034). A tint in the theme's surface colour keeps the contrast.
            let tint = match appearance.theme {
                crate::platform::UiTheme::Light => NSColor::colorWithWhite_alpha(1.0, 0.55),
                crate::platform::UiTheme::Dark => NSColor::colorWithWhite_alpha(0.0, 0.45),
            };
            view.setTintColor(Some(&tint));
            install(&content, &view, GLASS_RADIUS);
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
            follow_theme(&view, appearance.theme);
            install(&content, &view, PANEL_RADIUS);
            debug!("installed NSVisualEffectView ({material:?}) forced active");
        }
        Material::Opaque | Material::Acrylic => {
            // Nothing behind the webview: tokens.css paints the opaque surface. The window stays
            // non-opaque and clear so the rounded corners show no square window background.
            let color = NSColor::clearColor();
            ns_window.setBackgroundColor(Some(&color));
            ns_window.setOpaque(false);
            round_content(&content, PANEL_RADIUS);
            debug!("opaque surface (webview background)");
        }
    }
}

/// Pins the material view to the theme Verso shows (spec 010): with a Light/Dark override that
/// differs from the system, the material would otherwise follow the system and leave the page's
/// text on the wrong background. Only the material view changes; the webview keeps its own theme.
fn follow_theme(view: &NSView, theme: crate::platform::UiTheme) {
    let name = objc2_foundation::NSString::from_str(match theme {
        crate::platform::UiTheme::Dark => "NSAppearanceNameDarkAqua",
        crate::platform::UiTheme::Light => "NSAppearanceNameAqua",
    });
    unsafe {
        let named: *mut AnyObject =
            objc2::msg_send![objc2::class!(NSAppearance), appearanceNamed: &*name];
        if !named.is_null() {
            let _: () = objc2::msg_send![view, setAppearance: named];
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

/// Settings uses the system **sidebar** material behind the whole window: the content column paints
/// an opaque surface over it and the sidebar column stays translucent (R12, T038).
pub fn apply_settings(window: &WebviewWindow, appearance: &Appearance) {
    if appearance.reduce_transparency || appearance.high_contrast {
        let mut opaque = appearance.clone();
        opaque.material = Material::Opaque;
        apply(window, &opaque);
        return;
    }
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    let Some(mtm) = MainThreadMarker::new() else {
        warn!("sidebar material applied off the main thread; keeping the CSS fallback");
        return;
    };
    let ns_window: &NSWindow = unsafe { &*(pointer as *const NSWindow) };
    let Some(content) = ns_window.contentView() else {
        return;
    };

    remove_material_views(&content);

    let view = NSVisualEffectView::new(mtm);
    view.setMaterial(NSVisualEffectMaterial::Sidebar);
    view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    view.setState(NSVisualEffectState::FollowsWindowActiveState);
    follow_theme(&view, appearance.theme);
    install(&content, &view, 10.0);
    debug!("installed the sidebar material for Settings");
}
