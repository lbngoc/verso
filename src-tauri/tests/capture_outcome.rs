//! T027 — capture outcomes map to popup/quick/onboarding states and clipboard notices.

use verso_lib::core::capture::{
    outcome_to_popup_state, terminal_bypass_modifier, Notice, PopupState, TerminalModifier,
};
use verso_lib::platform::{CaptureFailure, CaptureMethod, CaptureOutcome, ClipboardState};

fn success(
    method: CaptureMethod,
    clipboard: ClipboardState,
    copy_attempted: bool,
) -> CaptureOutcome {
    CaptureOutcome {
        method,
        text: Some("hello".into()),
        clipboard,
        skipped_formats: Vec::new(),
        failure: None,
        frontmost_app: Some("com.apple.Safari".into()),
        duration_ms: 12,
        copy_attempted,
    }
}

#[test]
fn accessibility_never_touches_the_clipboard() {
    let outcome = success(
        CaptureMethod::Accessibility,
        ClipboardState::NotRestored,
        true,
    );
    let plan = outcome_to_popup_state(&outcome);
    assert_eq!(plan.clipboard, ClipboardState::Untouched);
    assert!(plan.notice.is_none());
    assert_eq!(plan.popup, PopupState::Loading);
}

#[test]
fn an_untouched_counter_means_the_clipboard_was_not_touched() {
    let outcome = success(CaptureMethod::Copy, ClipboardState::NotRestored, false);
    let plan = outcome_to_popup_state(&outcome);
    assert_eq!(plan.clipboard, ClipboardState::Untouched);
}

#[test]
fn a_partial_restore_reports_the_skipped_formats() {
    let mut outcome = success(CaptureMethod::Copy, ClipboardState::PartiallyRestored, true);
    outcome.skipped_formats = vec!["CF_BITMAP".into(), "com.example.promise".into()];
    let plan = outcome_to_popup_state(&outcome);
    assert_eq!(plan.clipboard, ClipboardState::PartiallyRestored);
    assert_eq!(plan.skipped_formats.len(), 2);
    match plan.notice {
        Some(Notice::PartiallyRestored { skipped_formats }) => {
            assert_eq!(skipped_formats.len(), 2);
        }
        other => panic!("expected a partial-restore notice, got {other:?}"),
    }
}

#[test]
fn nothing_selected_opens_the_quick_panel() {
    let outcome = CaptureOutcome::failure(CaptureFailure::NothingSelected, None, 40);
    let plan = outcome_to_popup_state(&outcome);
    assert_eq!(plan.popup, PopupState::Hidden);
    assert!(plan.open_quick);
    assert!(!plan.open_onboarding);
}

#[test]
fn a_secure_field_opens_the_quick_panel() {
    let outcome = CaptureOutcome::failure(CaptureFailure::SecureField, None, 40);
    let plan = outcome_to_popup_state(&outcome);
    assert_eq!(plan.popup, PopupState::Hidden);
    assert!(plan.open_quick);
}

#[test]
fn a_terminal_gets_the_right_bypass_modifier() {
    assert_eq!(
        terminal_bypass_modifier(Some("com.apple.Terminal")),
        Some(TerminalModifier::Option)
    );
    assert_eq!(
        terminal_bypass_modifier(Some("com.googlecode.iterm2")),
        Some(TerminalModifier::Option)
    );
    assert_eq!(
        terminal_bypass_modifier(Some("net.kovidgoyal.kitty")),
        Some(TerminalModifier::Shift)
    );
    assert_eq!(
        terminal_bypass_modifier(Some("dev.wez.wezterm")),
        Some(TerminalModifier::Shift)
    );
    assert_eq!(
        terminal_bypass_modifier(Some("com.mitchellh.ghostty")),
        Some(TerminalModifier::Shift)
    );
    assert_eq!(
        terminal_bypass_modifier(Some("com.chojm.kaku")),
        Some(TerminalModifier::Shift)
    );
    assert_eq!(terminal_bypass_modifier(Some("com.apple.Safari")), None);

    let outcome = CaptureOutcome::failure(
        CaptureFailure::TerminalTui,
        Some("com.googlecode.iterm2".into()),
        40,
    );
    let plan = outcome_to_popup_state(&outcome);
    assert_eq!(
        plan.popup,
        PopupState::CaptureFailed {
            failure: CaptureFailure::TerminalTui
        }
    );
    assert_eq!(plan.terminal_modifier, Some(TerminalModifier::Option));
}

#[test]
fn missing_permission_opens_onboarding() {
    let outcome = CaptureOutcome::failure(CaptureFailure::PermissionMissing, None, 3);
    let plan = outcome_to_popup_state(&outcome);
    assert!(plan.open_onboarding);
    assert_eq!(
        plan.popup,
        PopupState::CaptureFailed {
            failure: CaptureFailure::PermissionMissing
        }
    );
}

#[test]
fn elevated_windows_and_timeouts_are_explained_without_onboarding() {
    for failure in [CaptureFailure::ElevatedWindow, CaptureFailure::Timeout] {
        let outcome = CaptureOutcome::failure(failure, None, 400);
        let plan = outcome_to_popup_state(&outcome);
        assert_eq!(plan.popup, PopupState::CaptureFailed { failure });
        assert!(!plan.open_onboarding);
        assert!(!plan.open_quick);
    }
}
