//! Cursor position and monitor work area in logical points (multi-monitor, HiDPI aware).

use objc2::msg_send;
use objc2::rc::Retained;
use objc2_app_kit::NSScreen;
use objc2_foundation::NSRect;

use crate::platform::{Point, Rect};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CGPointF {
    pub x: f64,
    pub y: f64,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreate(source: *const std::ffi::c_void) -> *mut std::ffi::c_void;
    fn CGEventGetLocation(event: *mut std::ffi::c_void) -> CGPointF;
    fn CFRelease(value: *const std::ffi::c_void);
}

/// The pointer in global display coordinates (top-left origin, logical points).
pub fn cursor_position() -> Point {
    unsafe {
        let event = CGEventCreate(std::ptr::null());
        if event.is_null() {
            return Point { x: 0.0, y: 0.0 };
        }
        let location = CGEventGetLocation(event);
        CFRelease(event as *const std::ffi::c_void);
        Point {
            x: location.x,
            y: location.y,
        }
    }
}

fn screens() -> Vec<(NSRect, NSRect, f64)> {
    unsafe {
        let screens: Retained<objc2_foundation::NSArray<NSScreen>> =
            msg_send![objc2::class!(NSScreen), screens];
        screens
            .iter()
            .map(|screen| {
                let frame: NSRect = msg_send![&*screen, frame];
                let visible: NSRect = msg_send![&*screen, visibleFrame];
                let scale: f64 = msg_send![&*screen, backingScaleFactor];
                (frame, visible, scale)
            })
            .collect()
    }
}

fn main_screen_height() -> f64 {
    unsafe {
        let screen: Option<Retained<NSScreen>> = msg_send![objc2::class!(NSScreen), mainScreen];
        match screen {
            Some(screen) => {
                let frame: NSRect = msg_send![&*screen, frame];
                frame.size.height
            }
            None => 0.0,
        }
    }
}

/// Cocoa rects are bottom-left based; the rest of the app uses top-left global coordinates.
fn to_top_left(rect: NSRect, main_height: f64) -> Rect {
    Rect {
        x: rect.origin.x,
        y: main_height - rect.origin.y - rect.size.height,
        width: rect.size.width,
        height: rect.size.height,
    }
}

/// The visible frame (menu bar and Dock excluded) of the monitor that contains `point`.
pub fn work_area_for(point: Point) -> Rect {
    let screens = screens();
    if screens.is_empty() {
        return Rect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
        };
    }
    let main_height = main_screen_height();
    let converted: Vec<Rect> = screens
        .iter()
        .map(|(frame, _, _)| to_top_left(*frame, main_height))
        .collect();

    for (index, frame) in converted.iter().enumerate() {
        if frame.contains(point) {
            return to_top_left(screens[index].1, main_height);
        }
    }
    // Cursor off-screen (e.g. during a display change): use the screen nearest the point.
    let mut best = converted[0];
    let mut best_distance = f64::MAX;
    for (index, frame) in converted.iter().enumerate() {
        let dx = point.x - frame.x - frame.width / 2.0;
        let dy = point.y - frame.y - frame.height / 2.0;
        let distance = dx * dx + dy * dy;
        if distance < best_distance {
            best_distance = distance;
            best = to_top_left(screens[index].1, main_height);
        }
    }
    best
}

/// Scale factor of the monitor containing the point (for reference/tests; placement is logical).
pub fn scale_factor_for(point: Point) -> f64 {
    let screens = screens();
    let main_height = main_screen_height();
    for (frame, _, scale) in &screens {
        if to_top_left(*frame, main_height).contains(point) {
            return *scale;
        }
    }
    screens.first().map(|(_, _, scale)| *scale).unwrap_or(1.0)
}
