//! macOS Accessibility permission status, the one-time prompt, and the System Settings link.

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_foundation_sys::base::CFTypeRef;
use log::{info, warn};

use crate::platform::{PermissionKind, PermissionStatus, ResetOutcome};

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

/// The system's Accessibility prompt (adds Verso's row when it has none); Screen Recording has no
/// prompt here, so its pane opens instead.
fn prompt(kind: PermissionKind) {
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
        }
        PermissionKind::ScreenRecording => request(kind),
    }
}

/// Shows the system's one-time Accessibility prompt and opens the pane if it was already answered.
pub fn request(kind: PermissionKind) {
    match kind {
        PermissionKind::Accessibility => {
            prompt(kind);
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

/// "Reset and grant again" (spec 010 R2): removes only Verso's own TCC entry with `tccutil`, so a
/// stale row (switched on but bound to an older signature) cannot linger, then asks again so a fresh
/// row bound to the running build appears. Never run automatically.
pub fn reset(kind: PermissionKind) -> ResetOutcome {
    let granted = status(kind) == PermissionStatus::Granted;
    let outcome = reset_with(kind, granted, |args| {
        std::process::Command::new("/usr/bin/tccutil")
            .args(args)
            .status()
            .map(|status| status.code().unwrap_or(-1))
    });
    if !outcome.ok {
        return outcome;
    }
    // Only the system prompt: its own "Open System Settings" adds the fresh row and opens the
    // pane. Opening the pane here as well covered the prompt and showed a list without Verso
    // (spec 010 quickstart S5, macOS VM 1).
    prompt(kind);
    ResetOutcome {
        ok: true,
        status: status(kind),
    }
}

/// The decision part of [`reset`], with the process runner injected for tests.
pub fn reset_with(
    kind: PermissionKind,
    granted: bool,
    runner: impl Fn(&[&str]) -> std::io::Result<i32>,
) -> ResetOutcome {
    if granted {
        info!("permission reset refused: the grant is effective");
        return ResetOutcome {
            ok: false,
            status: PermissionStatus::Granted,
        };
    }
    let service = crate::core::settings::tcc_service(kind);
    let ok = match runner(&["reset", service, BUNDLE_ID]) {
        Ok(0) => {
            info!("tccutil reset {service} {BUNDLE_ID}: done");
            true
        }
        Ok(code) => {
            warn!("tccutil reset {service} {BUNDLE_ID} exited with {code}");
            false
        }
        Err(error) => {
            warn!("tccutil reset {service} {BUNDLE_ID} could not run: {error}");
            false
        }
    };
    ResetOutcome {
        ok,
        status: PermissionStatus::Denied,
    }
}

const BUNDLE_ID: &str = "app.versotranslate";

fn open_pane(url: &str) {
    info!("opening macOS privacy pane");
    let _ = std::process::Command::new("open").arg(url).status();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn reset_is_refused_while_the_grant_is_effective() {
        let calls = RefCell::new(0);
        let outcome = reset_with(PermissionKind::Accessibility, true, |_| {
            *calls.borrow_mut() += 1;
            Ok(0)
        });
        assert!(!outcome.ok);
        assert_eq!(outcome.status, PermissionStatus::Granted);
        assert_eq!(*calls.borrow(), 0);
    }

    #[test]
    fn reset_runs_tccutil_for_the_kind_and_bundle() {
        let seen = RefCell::new(Vec::new());
        let outcome = reset_with(PermissionKind::Accessibility, false, |args| {
            seen.borrow_mut()
                .extend(args.iter().map(|arg| arg.to_string()));
            Ok(0)
        });
        assert!(outcome.ok);
        assert_eq!(
            *seen.borrow(),
            vec!["reset", "Accessibility", "app.versotranslate"]
        );

        seen.borrow_mut().clear();
        reset_with(PermissionKind::ScreenRecording, false, |args| {
            seen.borrow_mut()
                .extend(args.iter().map(|arg| arg.to_string()));
            Ok(0)
        });
        assert_eq!(seen.borrow()[1], "ScreenCapture");
    }

    #[test]
    fn reset_reports_failure_on_a_non_zero_exit_or_an_error() {
        let failed = reset_with(PermissionKind::Accessibility, false, |_| Ok(1));
        assert!(!failed.ok);
        assert_eq!(failed.status, PermissionStatus::Denied);
        let missing = reset_with(PermissionKind::Accessibility, false, |_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no tccutil",
            ))
        });
        assert!(!missing.ok);
    }
}
