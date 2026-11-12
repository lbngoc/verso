//! Pure rules that turn a `CaptureOutcome` into what the user sees (FR-003/FR-018).
//!
//! Kept free of OS calls so the mapping can be tested exhaustively.

use serde::{Deserialize, Serialize};

use crate::core::errors::EngineError;
use crate::platform::{CaptureFailure, CaptureMethod, CaptureOutcome, ClipboardState};

/// `PopupState` (data-model): hidden → loading → result | error | capture_failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PopupState {
    Hidden,
    Loading,
    Result,
    Error { error: EngineError },
    CaptureFailed { failure: CaptureFailure },
}

/// A line shown with a result (popup-ui-contract § States › Notices).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Notice {
    Truncated {
        shown: usize,
        total: usize,
    },
    PartiallyRestored {
        skipped_formats: Vec<String>,
    },
    ClipboardNotRestored,
    Redirected {
        source: String,
        target: String,
    },
    Fallback {
        from: String,
        to: String,
        error: EngineError,
    },
    StillTranslating,
}

/// Which modifier lets a full-screen terminal program see the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalModifier {
    /// Terminal.app / iTerm2: hold Option while selecting.
    Option,
    /// WezTerm / kitty / Ghostty / Kaku: hold Shift while selecting.
    Shift,
}

pub fn terminal_bypass_modifier(frontmost_app: Option<&str>) -> Option<TerminalModifier> {
    let app = frontmost_app?.to_ascii_lowercase();
    let bundle_or_name = |needle: &str| app.contains(needle);
    if bundle_or_name("iterm2")
        || bundle_or_name("com.apple.terminal")
        || bundle_or_name("terminal.app")
    {
        return Some(TerminalModifier::Option);
    }
    if bundle_or_name("wezterm")
        || bundle_or_name("kitty")
        || bundle_or_name("ghostty")
        || bundle_or_name("kaku")
    {
        return Some(TerminalModifier::Shift);
    }
    None
}

/// Everything the UI needs after a capture attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturePlan {
    pub popup: PopupState,
    pub clipboard: ClipboardState,
    pub skipped_formats: Vec<String>,
    pub notice: Option<Notice>,
    pub open_quick: bool,
    pub open_onboarding: bool,
    /// Modifier hint when the frontmost app is a full-screen terminal.
    pub terminal_modifier: Option<TerminalModifier>,
}

pub fn outcome_to_popup_state(outcome: &CaptureOutcome) -> CapturePlan {
    // The clipboard is untouched whenever accessibility answered or the copy path never ran.
    let clipboard = if outcome.method == CaptureMethod::Accessibility || !outcome.copy_attempted {
        ClipboardState::Untouched
    } else {
        outcome.clipboard
    };

    let skipped_formats = if clipboard == ClipboardState::PartiallyRestored {
        outcome.skipped_formats.clone()
    } else {
        Vec::new()
    };

    let notice = match clipboard {
        ClipboardState::PartiallyRestored => Some(Notice::PartiallyRestored {
            skipped_formats: skipped_formats.clone(),
        }),
        ClipboardState::NotRestored => Some(Notice::ClipboardNotRestored),
        _ => None,
    };

    match outcome.failure {
        Some(CaptureFailure::NothingSelected) | Some(CaptureFailure::SecureField) => CapturePlan {
            popup: PopupState::Hidden,
            clipboard,
            skipped_formats,
            notice,
            open_quick: true,
            open_onboarding: false,
            terminal_modifier: None,
        },
        Some(failure) => CapturePlan {
            popup: PopupState::CaptureFailed { failure },
            clipboard,
            skipped_formats,
            notice,
            open_quick: false,
            open_onboarding: matches!(failure, CaptureFailure::PermissionMissing),
            terminal_modifier: if failure == CaptureFailure::TerminalTui {
                terminal_bypass_modifier(outcome.frontmost_app.as_deref())
                    .or(Some(TerminalModifier::Option))
            } else {
                None
            },
        },
        None => CapturePlan {
            popup: PopupState::Loading,
            clipboard,
            skipped_formats,
            notice,
            open_quick: false,
            open_onboarding: false,
            terminal_modifier: None,
        },
    }
}
