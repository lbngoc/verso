//! macOS appearance detection (research P7/P9): material, theme, Reduce Transparency, contrast.

use log::debug;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::NSString;

use crate::platform::{Appearance, Material, Os, UiTheme};

pub fn os_version() -> String {
    std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "13.0".into())
}

fn major_version(version: &str) -> u32 {
    version
        .split('.')
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or(13)
}

fn defaults_string(key: &str) -> Option<String> {
    unsafe {
        let defaults: Retained<AnyObject> =
            msg_send![objc2::class!(NSUserDefaults), standardUserDefaults];
        let key = NSString::from_str(key);
        let value: Option<Retained<NSString>> = msg_send![&*defaults, stringForKey: &*key];
        value.map(|value| value.to_string())
    }
}

fn defaults_integer(key: &str) -> Option<i64> {
    unsafe {
        let defaults: Retained<AnyObject> =
            msg_send![objc2::class!(NSUserDefaults), standardUserDefaults];
        let key = NSString::from_str(key);
        let value: i64 = msg_send![&*defaults, integerForKey: &*key];
        Some(value)
    }
}

fn workspace_flag(selector: &str) -> bool {
    unsafe {
        let workspace: Retained<AnyObject> = msg_send![objc2::class!(NSWorkspace), sharedWorkspace];
        if selector == "reduce" {
            msg_send![&*workspace, accessibilityDisplayShouldReduceTransparency]
        } else {
            msg_send![&*workspace, accessibilityDisplayShouldIncreaseContrast]
        }
    }
}

pub fn theme() -> UiTheme {
    match defaults_string("AppleInterfaceStyle").as_deref() {
        Some("Dark") => UiTheme::Dark,
        _ => UiTheme::Light,
    }
}

pub fn reduce_transparency() -> bool {
    workspace_flag("reduce")
}

pub fn increase_contrast() -> bool {
    workspace_flag("contrast")
}

pub fn accent() -> Option<String> {
    // AppleAccentColor: -1 graphite, 0 red, 1 orange, 2 yellow, 3 green, 4 blue, 5 purple, 6 pink.
    let value = defaults_integer("AppleAccentColor")?;
    let hex = match value {
        -1 => "#8e8e93",
        0 => "#ff5257",
        1 => "#ff9f0a",
        2 => "#ffd60a",
        3 => "#30d158",
        4 => "#0a84ff",
        5 => "#bf5af2",
        6 => "#ff375f",
        _ => return None,
    };
    Some(hex.into())
}

/// Test seam: `VERSO_FORCE_MATERIAL=opaque|vibrancy|glass` isolates the material cost.
fn forced_material() -> Option<Material> {
    match std::env::var("VERSO_FORCE_MATERIAL").ok()?.as_str() {
        "opaque" => Some(Material::Opaque),
        "vibrancy" => Some(Material::Vibrancy),
        "glass" => Some(Material::Glass),
        _ => None,
    }
}

pub fn current_appearance() -> Appearance {
    let os_version = os_version();
    let reduce_transparency = reduce_transparency();
    let high_contrast = increase_contrast();
    let material = if reduce_transparency || high_contrast {
        Material::Opaque
    } else if let Some(forced) = forced_material() {
        forced
    } else if major_version(&os_version) >= 26 {
        Material::Glass
    } else {
        Material::Vibrancy
    };
    let appearance = Appearance {
        os: Os::Macos,
        os_version,
        material,
        theme: theme(),
        reduce_transparency,
        high_contrast,
        accent: accent(),
    };
    debug!(
        "macOS appearance: {} / {:?} / {:?}",
        appearance.os_version, appearance.material, appearance.theme
    );
    appearance
}
