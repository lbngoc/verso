//! macOS selection capture: Accessibility first, simulated ⌘C second (research P4).
//!
//! Accessibility answers without touching the clipboard. The copy fallback waits for modifier
//! release, snapshots every pasteboard item, sends ⌘C with CGEvent, polls the change counter every
//! 5 ms for at most 400 ms, reads the text and restores the snapshot marked transient.

use std::ffi::c_void;
use std::thread::sleep;
use std::time::{Duration, Instant};

use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation_sys::base::CFTypeRef;
use log::debug;
use objc2_app_kit::NSWorkspace;

use super::{pasteboard, permissions};
use crate::platform::{CaptureFailure, CaptureMethod, CaptureOutcome, ClipboardState};

type AXUIElementRef = CFTypeRef;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> i32;
    fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, timeout: f32) -> i32;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreateKeyboardEvent(
        source: *const c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut c_void;
    fn CGEventSetFlags(event: *mut c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut c_void);
    fn CGEventSourceCreate(state_id: i32) -> *mut c_void;
    fn CGEventSourceFlagsState(state_id: i32) -> u64;
    fn CFRelease(value: *const c_void);
}

const KEY_C: u16 = 0x08;
const FLAG_COMMAND: u64 = 1 << 20;
const MODIFIER_MASK: u64 = (1 << 17) | (1 << 18) | (1 << 19) | (1 << 20) | (1 << 23);
/// kCGEventSourceStateCombinedSessionState
const SOURCE_COMBINED_SESSION: i32 = 0;
/// kCGHIDEventTap
const TAP_HID: u32 = 0;

const TERMINAL_OPTION_APPS: [&str; 2] = ["com.apple.Terminal", "com.googlecode.iterm2"];
const TERMINAL_APPS: [&str; 6] = [
    "com.apple.Terminal",
    "com.googlecode.iterm2",
    "dev.wez.wezterm",
    "net.kovidgoyal.kitty",
    "com.mitchellh.ghostty",
    "com.chojm.kaku",
];

pub fn frontmost_app() -> Option<String> {
    let workspace = NSWorkspace::sharedWorkspace();
    workspace
        .frontmostApplication()
        .and_then(|app| app.bundleIdentifier())
        .map(|bundle| bundle.to_string())
}

fn is_terminal(app: Option<&str>) -> bool {
    let Some(app) = app else { return false };
    TERMINAL_APPS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(app))
        || TERMINAL_OPTION_APPS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(app))
}

fn copy_attribute(element: AXUIElementRef, name: &str) -> Option<CFType> {
    let attribute = CFString::new(name);
    let mut value: CFTypeRef = std::ptr::null();
    let status = unsafe {
        AXUIElementCopyAttributeValue(element, attribute.as_concrete_TypeRef(), &mut value)
    };
    if status == 0 && !value.is_null() {
        Some(unsafe { CFType::wrap_under_create_rule(value) })
    } else {
        None
    }
}

fn string_attribute(element: AXUIElementRef, name: &str) -> Option<String> {
    copy_attribute(element, name)
        .and_then(|value| value.downcast::<CFString>())
        .map(|value| value.to_string())
}

fn focused_element(system: AXUIElementRef) -> Option<CFType> {
    copy_attribute(system, "AXFocusedUIElement")
}

fn ax_selected_text(system: AXUIElementRef) -> Option<String> {
    let focused = focused_element(system)?;
    string_attribute(focused.as_CFTypeRef(), "AXSelectedText")
}

fn ax_secure_field(system: AXUIElementRef) -> bool {
    if let Some(focused) = focused_element(system) {
        if string_attribute(focused.as_CFTypeRef(), "AXSubrole").as_deref()
            == Some("AXSecureTextField")
        {
            return true;
        }
    }
    false
}

fn flag_manual_accessibility(system: AXUIElementRef) {
    if let Some(focused) = focused_element(system) {
        let attribute = CFString::new("AXManualAccessibility");
        let value = CFBoolean::true_value();
        unsafe {
            AXUIElementSetAttributeValue(
                focused.as_CFTypeRef(),
                attribute.as_concrete_TypeRef(),
                value.as_CFTypeRef(),
            );
        }
    }
}

fn wait_for_modifier_release(timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let flags = unsafe { CGEventSourceFlagsState(SOURCE_COMBINED_SESSION) };
        if flags & MODIFIER_MASK == 0 {
            return;
        }
        sleep(Duration::from_millis(10));
    }
}

fn send_copy() {
    unsafe {
        let source = CGEventSourceCreate(SOURCE_COMBINED_SESSION);
        let key_down = CGEventCreateKeyboardEvent(source, KEY_C, true);
        let key_up = CGEventCreateKeyboardEvent(source, KEY_C, false);
        if !key_down.is_null() {
            CGEventSetFlags(key_down, FLAG_COMMAND);
            CGEventPost(TAP_HID, key_down);
            CFRelease(key_down as *const c_void);
        }
        if !key_up.is_null() {
            CGEventSetFlags(key_up, FLAG_COMMAND);
            CGEventPost(TAP_HID, key_up);
            CFRelease(key_up as *const c_void);
        }
        if !source.is_null() {
            CFRelease(source as *const c_void);
        }
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis() as u64
}

/// Captures the current selection; the returned outcome never contains the text in logs.
pub fn capture_selection() -> CaptureOutcome {
    let started = Instant::now();
    let app = frontmost_app();

    if !permissions::accessibility_granted() {
        return CaptureOutcome::failure(
            CaptureFailure::PermissionMissing,
            app,
            elapsed_ms(started),
        );
    }

    let system = unsafe { AXUIElementCreateSystemWide() };
    if system.is_null() {
        return copy_fallback(app, started);
    }
    unsafe { AXUIElementSetMessagingTimeout(system, 0.15) };
    let system = unsafe { CFType::wrap_under_create_rule(system) };
    let system_ref = system.as_CFTypeRef();

    if let Some(text) = ax_selected_text(system_ref) {
        if !text.trim().is_empty() {
            debug!("selection captured through accessibility");
            return CaptureOutcome::success(
                CaptureMethod::Accessibility,
                text,
                elapsed_ms(started),
            );
        }
    }

    // Some apps expose their selection only after the manual-accessibility hint (one retry).
    flag_manual_accessibility(system_ref);
    if let Some(text) = ax_selected_text(system_ref) {
        if !text.trim().is_empty() {
            debug!("selection captured through accessibility after AXManualAccessibility");
            return CaptureOutcome::success(
                CaptureMethod::Accessibility,
                text,
                elapsed_ms(started),
            );
        }
    }

    if ax_secure_field(system_ref) {
        debug!("focused element is a secure text field");
        return CaptureOutcome::failure(CaptureFailure::SecureField, app, elapsed_ms(started));
    }

    copy_fallback(app, started)
}

fn copy_fallback(app: Option<String>, started: Instant) -> CaptureOutcome {
    wait_for_modifier_release(Duration::from_millis(400));

    let snapshot = pasteboard::PasteboardSnapshot::capture();
    let baseline = pasteboard::change_count();
    send_copy();

    let deadline = Instant::now() + Duration::from_millis(400);
    let mut changed = false;
    while Instant::now() < deadline {
        if pasteboard::change_count() != baseline {
            changed = true;
            break;
        }
        sleep(Duration::from_millis(5));
    }
    sleep(Duration::from_millis(10));

    let text = if changed {
        pasteboard::read_text()
    } else {
        None
    };

    let (clipboard, skipped_formats) = match snapshot {
        Some(snapshot) => {
            let report = snapshot.restore(pasteboard::change_count());
            (report.state, report.skipped_formats)
        }
        None => (
            ClipboardState::NotRestored,
            vec!["pasteboard snapshot".to_string()],
        ),
    };

    match text {
        Some(text) if !text.trim().is_empty() => {
            let mut outcome = CaptureOutcome {
                method: CaptureMethod::Copy,
                text: Some(text),
                clipboard,
                skipped_formats,
                failure: None,
                frontmost_app: app,
                duration_ms: elapsed_ms(started),
                copy_attempted: true,
            };
            if outcome.clipboard == ClipboardState::Restored && !outcome.skipped_formats.is_empty()
            {
                outcome.clipboard = ClipboardState::PartiallyRestored;
            }
            outcome
        }
        _ => {
            if is_terminal(app.as_deref()) {
                CaptureOutcome {
                    method: CaptureMethod::Copy,
                    text: None,
                    clipboard,
                    skipped_formats,
                    failure: Some(CaptureFailure::TerminalTui),
                    frontmost_app: app,
                    duration_ms: elapsed_ms(started),
                    copy_attempted: true,
                }
            } else {
                CaptureOutcome {
                    method: CaptureMethod::Copy,
                    text: None,
                    clipboard,
                    skipped_formats,
                    failure: Some(CaptureFailure::NothingSelected),
                    frontmost_app: app,
                    duration_ms: elapsed_ms(started),
                    copy_attempted: true,
                }
            }
        }
    }
}
