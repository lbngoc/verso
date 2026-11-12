//! Windows material and window chrome: DWM rounded corners, flyout shadow, dark mode and the
//! Acrylic backdrop only when the M0 probe passed (popup-ui-contract § appearance matrix).

use log::debug;
use tauri::WebviewWindow;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMSBT_TRANSIENTWINDOW, DWMWA_BORDER_COLOR, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DWMWINDOWATTRIBUTE,
};

use super::appearance::build_number;
use crate::platform::{Appearance, Material, UiTheme};

fn hwnd_of(window: &WebviewWindow) -> Option<HWND> {
    window.hwnd().ok().map(|hwnd| HWND(hwnd.0))
}

fn set_attribute(hwnd: HWND, attribute: DWMWINDOWATTRIBUTE, value: &[u8]) {
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            attribute,
            value.as_ptr() as *const core::ffi::c_void,
            value.len() as u32,
        );
    }
}

pub fn apply(window: &WebviewWindow, appearance: &Appearance) {
    let Some(hwnd) = hwnd_of(window) else {
        return;
    };
    let build = build_number();

    if build >= 22_000 {
        let corners = DWMWCP_ROUND.0.to_ne_bytes();
        set_attribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &corners);
    }

    let dark: i32 = if appearance.theme == UiTheme::Dark {
        1
    } else {
        0
    };
    set_attribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark.to_ne_bytes());

    if build >= 22_621 {
        let backdrop: i32 = match appearance.material {
            Material::Acrylic => DWMSBT_TRANSIENTWINDOW.0,
            // DWMSBT_NONE: the stylesheet paints the flyout surface (what Windows shows when an
            // unfocused acrylic window falls back).
            _ => 1,
        };
        set_attribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, &backdrop.to_ne_bytes());
    }

    // A hairline SurfaceStroke is drawn by the stylesheet; the DWM border stays out of the way.
    let border: u32 = 0xFFFF_FFFF; // DWMWA_COLOR_NONE
    set_attribute(hwnd, DWMWA_BORDER_COLOR, &border.to_ne_bytes());

    debug!(
        "applied DWM attributes (build {build}, material {:?})",
        appearance.material
    );
}
