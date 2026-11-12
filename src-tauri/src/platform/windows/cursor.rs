//! Cursor position and monitor work area in logical points on Windows (HiDPI aware).

use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::platform::{Point, Rect};

pub fn cursor_position() -> Point {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        return Point { x: 0.0, y: 0.0 };
    }
    let monitor = unsafe {
        MonitorFromPoint(
            POINT {
                x: point.x,
                y: point.y,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    };
    let scale = scale_for_monitor(monitor);
    Point {
        x: point.x as f64 / scale,
        y: point.y as f64 / scale,
    }
}

fn scale_for_monitor(monitor: windows::Win32::Graphics::Gdi::HMONITOR) -> f64 {
    let mut dpi_x = 96u32;
    let mut dpi_y = 96u32;
    let ok = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    if ok.is_ok() && dpi_x > 0 {
        dpi_x as f64 / 96.0
    } else {
        1.0
    }
}

/// Logical work area (taskbar excluded) of the monitor containing `point` (logical points).
pub fn work_area_for(point: Point) -> Rect {
    // The caller's point is logical; Win32 wants physical pixels to pick the monitor.
    let initial = POINT {
        x: point.x.round() as i32,
        y: point.y.round() as i32,
    };
    let monitor = unsafe { MonitorFromPoint(initial, MONITOR_DEFAULTTONEAREST) };
    let scale = scale_for_monitor(monitor);
    let physical_point = POINT {
        x: (point.x * scale).round() as i32,
        y: (point.y * scale).round() as i32,
    };
    let monitor = unsafe { MonitorFromPoint(physical_point, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        Rect {
            x: info.rcWork.left as f64 / scale,
            y: info.rcWork.top as f64 / scale,
            width: (info.rcWork.right - info.rcWork.left) as f64 / scale,
            height: (info.rcWork.bottom - info.rcWork.top) as f64 / scale,
        }
    } else {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 1280.0,
            height: 720.0,
        }
    }
}

pub fn scale_factor_for(point: Point) -> f64 {
    let physical = POINT {
        x: point.x.round() as i32,
        y: point.y.round() as i32,
    };
    let monitor = unsafe { MonitorFromPoint(physical, MONITOR_DEFAULTTONEAREST) };
    scale_for_monitor(monitor)
}
