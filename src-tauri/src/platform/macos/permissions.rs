//! macOS Accessibility permission status, the one-time prompt, and the System Settings link.

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_foundation_sys::base::CFTypeRef;
use log::info;

use crate::platform::{PermissionKind, PermissionStatus};

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
    fn CGPreflightScreenCaptureAccess() -> bool;
}

pub fn accessibility_granted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn status(kind: PermissionKind) -> PermissionStatus {
    let granted = match kind {
        PermissionKind::Accessibility => accessibility_granted(),
        PermissionKind::ScreenRecording => unsafe { CGPreflightScreenCaptureAccess() },
    };
    if granted {
        PermissionStatus::Granted
    } else {
        // macOS does not distinguish "denied" from "never asked" without a prompt.
        PermissionStatus::Denied
    }
}

/// Shows the system's one-time Accessibility prompt and opens the pane if it was already answered.
pub fn request(kind: PermissionKind) {
    match kind {
        PermissionKind::Accessibility => {
            let key = CFString::new("AXTrustedCheckOptionPrompt");
            let options = CFDictionary::from_CFType_pairs(&[(
                key.as_CFType(),
                CFBoolean::true_value().as_CFType(),
            )]);
            unsafe {
                AXIsProcessTrustedWithOptions(options.as_CFTypeRef());
            }
            open_pane(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            );
        }
        PermissionKind::ScreenRecording => {
            open_pane(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
            );
        }
    }
}

fn open_pane(url: &str) {
    info!("opening macOS privacy pane");
    let _ = std::process::Command::new("open").arg(url).status();
}
