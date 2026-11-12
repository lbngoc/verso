//! Windows appearance detection (research P8/P9): build, theme, transparency, high contrast, accent.

use log::debug;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

use crate::platform::{Appearance, Material, Os, UiTheme};

/// Turned on by the M0 Acrylic probe (T012) only if DWM draws the backdrop for the never-activated
/// popup. Until the probe passes on Windows 11, the popup uses the opaque Fluent flyout surface,
/// which is what Windows itself shows for an inactive acrylic window.
pub const ACRYLIC_PROBE_PASSED: bool = true;

fn read_dword(root: winreg::HKEY, path: &str, value: &str) -> Option<u32> {
    let key = RegKey::predef(root).open_subkey(path).ok()?;
    key.get_value::<u32, _>(value).ok()
}

fn read_string(root: winreg::HKEY, path: &str, value: &str) -> Option<String> {
    let key = RegKey::predef(root).open_subkey(path).ok()?;
    key.get_value::<String, _>(value).ok()
}

pub fn build_number() -> u32 {
    read_string(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "CurrentBuildNumber",
    )
    .and_then(|value| value.parse().ok())
    .unwrap_or(10_000)
}

pub fn os_version() -> String {
    let build = build_number();
    let ubr = read_dword(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "UBR",
    )
    .unwrap_or(0);
    format!("10.0.{build}.{ubr}")
}

fn personalize_dword(name: &str) -> Option<u32> {
    read_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
        name,
    )
}

pub fn theme() -> UiTheme {
    match personalize_dword("AppsUseLightTheme") {
        Some(0) => UiTheme::Dark,
        Some(_) => UiTheme::Light,
        // Absent on older builds: follow the system default (light).
        None => UiTheme::Light,
    }
}

/// Whether the taskbar and notification area are dark. Windows themes them separately from apps
/// (`SystemUsesLightTheme`, not `AppsUseLightTheme`), and the default on Windows 10 and 11 is dark.
///
/// High-contrast themes repaint the taskbar but leave `SystemUsesLightTheme` at the normal theme's
/// value, so the decision moves to the theme background's luminance (research R9).
pub fn taskbar_is_dark() -> bool {
    let high_contrast = high_contrast();
    let background = if high_contrast {
        // COLORREF is 0x00BBGGRR.
        let color = unsafe {
            windows::Win32::Graphics::Gdi::GetSysColor(windows::Win32::Graphics::Gdi::COLOR_WINDOW)
        };
        Some((
            (color & 0xFF) as u8,
            ((color >> 8) & 0xFF) as u8,
            ((color >> 16) & 0xFF) as u8,
        ))
    } else {
        None
    };
    crate::tray::dark_bar(
        high_contrast,
        background,
        personalize_dword("SystemUsesLightTheme"),
    )
}

pub fn transparency_enabled() -> bool {
    personalize_dword("EnableTransparency")
        .map(|value| value != 0)
        .unwrap_or(true)
}

pub fn high_contrast() -> bool {
    read_string(
        HKEY_CURRENT_USER,
        r"Control Panel\Accessibility\HighContrast",
        "Flags",
    )
    .and_then(|flags| flags.parse::<u32>().ok())
    .map(|flags| flags & 0x1 != 0)
    .unwrap_or(false)
}

pub fn accent() -> Option<String> {
    // AccentColor is stored as AABBGGRR.
    let value = read_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\DWM",
        "AccentColor",
    )?;
    let red = value & 0xFF;
    let green = (value >> 8) & 0xFF;
    let blue = (value >> 16) & 0xFF;
    Some(format!("#{red:02x}{green:02x}{blue:02x}"))
}

pub fn current_appearance() -> Appearance {
    let os_version = os_version();
    let build = build_number();
    let high_contrast = high_contrast();
    let transparency = transparency_enabled();
    let material = if high_contrast || !transparency {
        Material::Opaque
    } else if build >= 22_621 && ACRYLIC_PROBE_PASSED {
        Material::Acrylic
    } else {
        Material::Opaque
    };
    let appearance = Appearance {
        os: Os::Windows,
        os_version,
        material,
        theme: theme(),
        reduce_transparency: !transparency,
        high_contrast,
        accent: accent(),
    };
    debug!(
        "Windows appearance: build {} / {:?} / {:?}",
        build, appearance.material, appearance.theme
    );
    appearance
}
