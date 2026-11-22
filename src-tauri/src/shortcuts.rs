//! Global shortcuts from settings, re-registered when settings change; conflicts are reported to
//! the UI so Settings can ask for another combination (FR-009, research P6; spec 009 R6).

use log::{info, warn};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::commands::{self, AppState};
use crate::core::settings::SettingsProfile;
use crate::platform;
use crate::{tray, window_manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Translate,
    Speak,
    Quick,
    Screen,
}

impl Action {
    pub fn id(self) -> &'static str {
        match self {
            Action::Translate => "translate",
            Action::Speak => "speak",
            Action::Quick => "quick",
            Action::Screen => "screen",
        }
    }

    pub fn default_accel(self) -> &'static str {
        match self {
            Action::Translate => "Alt+Shift+Z",
            Action::Speak => "Alt+Shift+S",
            Action::Quick => crate::core::settings::default_quick_shortcut(),
            Action::Screen => crate::core::settings::default_screen_shortcut(),
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "translate" => Some(Action::Translate),
            "speak" => Some(Action::Speak),
            "quick" => Some(Action::Quick),
            "screen" => Some(Action::Screen),
            _ => None,
        }
    }
}

/// Why a recorded shortcut was refused (data-model `BindingError`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum BindingError {
    NoModifier,
    ShiftOnly,
    Duplicate {
        action: String,
    },
    Reserved,
    /// The OS refused registration; reported after `register_all`.
    Taken {
        detail: String,
    },
}

const MODIFIERS: [&str; 5] = ["Cmd", "Super", "Ctrl", "Alt", "Shift"];

fn normalise_modifier(name: &str) -> Option<&'static str> {
    match name {
        "Cmd" | "Command" | "Meta" => Some(if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Super"
        }),
        "Super" | "Win" | "Windows" => Some("Super"),
        "Ctrl" | "Control" => Some("Ctrl"),
        "Alt" | "Option" => Some("Alt"),
        "Shift" => Some("Shift"),
        _ => None,
    }
}

/// Splits `Alt+Shift+T` into ordered modifiers and a key, or `None` when it does not parse.
fn split_accelerator(accel: &str) -> Option<(Vec<&'static str>, String)> {
    let parts: Vec<&str> = accel
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return Some((Vec::new(), String::new()));
    }
    let key = parts[parts.len() - 1].to_string();
    let mut modifiers = Vec::new();
    for part in &parts[..parts.len() - 1] {
        let modifier = normalise_modifier(part)?;
        if !modifiers.contains(&modifier) {
            modifiers.push(modifier);
        }
    }
    modifiers.sort_by_key(|modifier| {
        MODIFIERS
            .iter()
            .position(|known| known == modifier)
            .unwrap_or(MODIFIERS.len())
    });
    Some((modifiers, key))
}

fn reserved_list() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &[
            "Cmd+Q",
            "Cmd+W",
            "Cmd+H",
            "Cmd+M",
            "Cmd+Tab",
            "Cmd+Space",
            "Cmd+Shift+3",
            "Cmd+Shift+4",
            "Cmd+Shift+5",
            "Cmd+Ctrl+Q",
            "Cmd+Alt+Esc",
        ]
    } else {
        &[
            "Alt+F4",
            "Alt+Tab",
            "Super+L",
            "Super+D",
            "Super+E",
            "Super+Tab",
            "Ctrl+Alt+Delete",
            "Ctrl+Shift+Esc",
        ]
    }
}

/// Validates and normalises a recorded accelerator (pure; `""` clears the binding).
///
/// `others` holds the other actions' bindings so a duplicate can be named.
pub fn validate_binding(
    _action: &str,
    accel: &str,
    others: &[(&str, &str)],
) -> Result<String, BindingError> {
    if accel.trim().is_empty() {
        return Ok(String::new());
    }
    let Some((modifiers, key)) = split_accelerator(accel) else {
        return Err(BindingError::Reserved);
    };
    let has_command = modifiers
        .iter()
        .any(|modifier| matches!(*modifier, "Cmd" | "Super" | "Ctrl" | "Alt"));
    if modifiers.is_empty() {
        return Err(BindingError::NoModifier);
    }
    if !has_command {
        return Err(BindingError::ShiftOnly);
    }
    if key.is_empty() {
        return Err(BindingError::NoModifier);
    }
    let normalised = modifiers
        .iter()
        .copied()
        .chain(std::iter::once(key.as_str()))
        .collect::<Vec<_>>()
        .join("+");

    if reserved_list()
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(&normalised))
    {
        return Err(BindingError::Reserved);
    }
    for (other_action, other_accel) in others {
        if other_accel.trim().is_empty() {
            continue;
        }
        let Some((other_modifiers, other_key)) = split_accelerator(other_accel) else {
            continue;
        };
        let other = other_modifiers
            .iter()
            .copied()
            .chain(std::iter::once(other_key.as_str()))
            .collect::<Vec<_>>()
            .join("+");
        if other.eq_ignore_ascii_case(&normalised) {
            return Err(BindingError::Duplicate {
                action: (*other_action).to_string(),
            });
        }
    }
    Ok(normalised)
}

/// The actions that get a global shortcut: Translate, Speak and Quick always, Screen only while
/// the module is on, available on this system and bound (spec 004 R9). Pure.
pub fn active_actions(profile: &SettingsProfile, ocr_available: bool) -> Vec<Action> {
    let mut actions = vec![Action::Translate, Action::Speak, Action::Quick];
    if profile.ocr.enabled && ocr_available && !profile.shortcuts.screen.trim().is_empty() {
        actions.push(Action::Screen);
    }
    actions
}

fn binding_of(profile: &SettingsProfile, action: Action) -> &str {
    match action {
        Action::Translate => &profile.shortcuts.translate,
        Action::Speak => &profile.shortcuts.speak,
        Action::Quick => &profile.shortcuts.quick,
        Action::Screen => &profile.shortcuts.screen,
    }
}

pub fn register_all(app: &AppHandle) -> Result<(), String> {
    let settings = app.state::<AppState>().settings.get();
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();

    let mut conflict = None;
    let actions = active_actions(&settings, platform::services().supports("ocr"));
    for action in actions {
        let binding = binding_of(&settings, action).to_string();
        let result = register_one(app, &binding, action);
        if let Err(error) = result {
            if conflict.is_none() {
                conflict = Some((binding, error.clone()));
            }
            warn!("{error}");
        }
    }
    if let Some((shortcut, reason)) = conflict {
        let _ = app.emit(
            "shortcut-conflict",
            serde_json::json!({ "shortcut": shortcut, "reason": reason }),
        );
        return Err(reason);
    }
    info!("global shortcuts registered");
    Ok(())
}

/// Unregisters every global shortcut while the recorder is listening, then restores them (R6).
pub fn suspend(app: &AppHandle) {
    let _ = app.global_shortcut().unregister_all();
    info!("global shortcuts suspended for the recorder");
}

pub fn resume(app: &AppHandle) -> Result<(), String> {
    register_all(app)
}

fn register_one(app: &AppHandle, value: &str, action: Action) -> Result<(), String> {
    // An empty binding means "unbound" (spec 009): nothing to register and no conflict to report.
    if value.trim().is_empty() {
        return Ok(());
    }
    let shortcut: Shortcut = value
        .parse()
        .map_err(|_| format!("'{value}' is not a valid shortcut"))?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                match action {
                    Action::Translate => trigger_translate(app),
                    Action::Speak => trigger_speak(app),
                    Action::Quick => window_manager::toggle_quick(app),
                    Action::Screen => trigger_screen(app),
                }
            }
        })
        .map_err(|error| format!("'{value}' is already taken: {error}"))
}

/// Translate and Speak do nothing while the quick panel is open (FR-035). Never applied to Quick.
pub fn should_ignore_trigger(quick_open: bool) -> bool {
    quick_open
}

/// What the Translate shortcut does first: with the quick panel open on screen it closes the
/// panel and reads nothing (FR-035, Revision 2); otherwise it runs the usual capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslateAction {
    Close,
    Run,
}

pub fn translate_action(quick_open: bool) -> TranslateAction {
    if should_ignore_trigger(quick_open) {
        TranslateAction::Close
    } else {
        TranslateAction::Run
    }
}

pub fn trigger_translate(app: &AppHandle) {
    // Setup is the only window while it is open (FR-052).
    if window_manager::focus_setup_if_open(app) {
        return;
    }
    if translate_action(window_manager::quick_is_open(app)) == TranslateAction::Close {
        log::debug!("translate closes the open quick panel");
        window_manager::hide_quick(app);
        return;
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tray::set_busy(&handle, true);
        let state = handle.state::<AppState>();
        // Success or failure, the mark goes back to idle (spec 008 FR-010).
        let _ = commands::translate_selection(handle.clone(), state).await;
        tray::set_busy(&handle, false);
    });
}

/// The screen translate shortcut, the tray item and the "try another region" button. Inside the
/// quick panel or Setup it follows spec 009: Setup comes to the front, the panel closes, and no
/// overlay opens.
pub fn trigger_screen(app: &AppHandle) {
    if window_manager::focus_setup_if_open(app) {
        return;
    }
    if window_manager::quick_is_open(app) {
        log::debug!("screen translate closes the open quick panel");
        window_manager::hide_quick(app);
        return;
    }
    // Off the event-loop thread: creating the overlay windows from a hotkey handler can deadlock
    // on Windows while WebView2 waits for that very loop (seen in Windows 11 1, spec 004).
    let handle = app.clone();
    std::thread::spawn(move || {
        crate::core::ocr::start(&handle);
    });
}

pub fn trigger_speak(app: &AppHandle) {
    if should_ignore_trigger(window_manager::quick_is_open(app)) {
        log::debug!("speak ignored: the quick panel is open");
        return;
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<AppState>();
        let _ = commands::speak_selection(handle.clone(), state).await;
    });
}

/// Sets one binding: validate → save → re-register, restoring the old value when the OS refuses.
pub fn set_binding(app: &AppHandle, action: &str, accel: &str) -> Result<(), BindingError> {
    let Some(action) = Action::from_id(action) else {
        return Err(BindingError::Reserved);
    };
    let state = app.state::<AppState>();
    let profile = state.settings.get();
    let others = [
        ("translate", profile.shortcuts.translate.as_str()),
        ("speak", profile.shortcuts.speak.as_str()),
        ("quick", profile.shortcuts.quick.as_str()),
        ("screen", profile.shortcuts.screen.as_str()),
    ]
    .into_iter()
    .filter(|(other, _)| *other != action.id())
    .collect::<Vec<_>>();

    let normalised = validate_binding(action.id(), accel, &others)?;
    // The store merges a JSON object literally: the patch must nest under `shortcuts`, not use a
    // dotted key (which would become an unknown field and change nothing).
    let nested = |value: &str| match action {
        Action::Translate => serde_json::json!({ "shortcuts": { "translate": value } }),
        Action::Speak => serde_json::json!({ "shortcuts": { "speak": value } }),
        Action::Quick => serde_json::json!({ "shortcuts": { "quick": value } }),
        Action::Screen => serde_json::json!({ "shortcuts": { "screen": value } }),
    };
    state
        .settings
        .update(&nested(&normalised))
        .map_err(|error| BindingError::Taken {
            detail: error.to_string(),
        })?;
    if let Err(error) = register_all(app) {
        // The OS refused this combination: put the previous value back and register it again.
        let previous = match action {
            Action::Translate => profile.shortcuts.translate.clone(),
            Action::Speak => profile.shortcuts.speak.clone(),
            Action::Quick => profile.shortcuts.quick.clone(),
            Action::Screen => profile.shortcuts.screen.clone(),
        };
        let _ = state.settings.update(&nested(&previous));
        let _ = register_all(app);
        return Err(BindingError::Taken { detail: error });
    }
    let profile = state.settings.get();
    let _ = app.emit("settings-changed", &profile);
    Ok(())
}

/// Restores one binding to its platform default (FR-010).
pub fn reset_binding(app: &AppHandle, action: &str) -> Result<(), BindingError> {
    let Some(action) = Action::from_id(action) else {
        return Err(BindingError::Reserved);
    };
    set_binding(app, action.id(), action.default_accel())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_key_or_shift_alone_is_refused() {
        assert_eq!(
            validate_binding("translate", "T", &[]),
            Err(BindingError::NoModifier)
        );
        assert_eq!(
            validate_binding("translate", "Shift+T", &[]),
            Err(BindingError::ShiftOnly)
        );
    }

    #[test]
    fn another_action_cannot_share_a_binding() {
        assert_eq!(
            validate_binding("speak", "Alt+Shift+T", &[("translate", "Alt+Shift+T")]),
            Err(BindingError::Duplicate {
                action: "translate".into()
            })
        );
    }

    #[test]
    fn system_combinations_are_reserved() {
        let cases: &[&str] = if cfg!(target_os = "macos") {
            &["Cmd+Q", "Cmd+Space", "Cmd+Shift+4"]
        } else {
            &["Alt+F4", "Super+L", "Ctrl+Alt+Delete"]
        };
        for accel in cases {
            assert_eq!(
                validate_binding("translate", accel, &[]),
                Err(BindingError::Reserved),
                "{accel}"
            );
        }
    }

    #[test]
    fn a_valid_binding_is_normalised_in_modifier_order() {
        assert_eq!(
            validate_binding("translate", "Shift+Alt+Y", &[]),
            Ok("Alt+Shift+Y".into())
        );
        // Cmd is printed first on macOS, Super on Windows.
        let expected = if cfg!(target_os = "macos") {
            "Cmd+Alt+Shift+T"
        } else {
            "Super+Alt+Shift+T"
        };
        assert_eq!(
            validate_binding("quick", "Shift+Alt+Meta+T", &[]),
            Ok(expected.into())
        );
    }

    #[test]
    fn translate_and_speak_are_ignored_only_while_the_panel_is_open() {
        assert!(!should_ignore_trigger(false));
        assert!(should_ignore_trigger(true));
    }

    #[test]
    fn translate_closes_the_panel_only_while_it_is_open() {
        assert_eq!(translate_action(false), TranslateAction::Run);
        assert_eq!(translate_action(true), TranslateAction::Close);
    }

    // ---- spec 004 (T005) ----

    #[test]
    fn the_screen_action_has_its_id_and_default() {
        assert_eq!(Action::Screen.id(), "screen");
        assert_eq!(Action::Screen.default_accel(), "Alt+Shift+X");
        assert_eq!(Action::from_id("screen"), Some(Action::Screen));
    }

    #[test]
    fn the_screen_binding_cannot_equal_another_action() {
        for other in ["translate", "speak", "quick"] {
            assert_eq!(
                validate_binding("screen", "Alt+Shift+X", &[(other, "Alt+Shift+X")]),
                Err(BindingError::Duplicate {
                    action: other.into()
                })
            );
        }
        assert_eq!(validate_binding("screen", "", &[]), Ok(String::new()));
    }

    #[test]
    fn the_screen_action_is_active_only_while_the_module_is_on_and_available() {
        let mut profile = crate::core::settings::SettingsProfile::default();
        assert!(!active_actions(&profile, true).contains(&Action::Screen));
        profile.ocr.enabled = true;
        assert!(active_actions(&profile, true).contains(&Action::Screen));
        assert!(!active_actions(&profile, false).contains(&Action::Screen));
        profile.shortcuts.screen.clear();
        assert!(!active_actions(&profile, true).contains(&Action::Screen));
        // The other three are always active while bound.
        assert!(active_actions(&profile, true).contains(&Action::Translate));
        assert!(active_actions(&profile, false).contains(&Action::Quick));
    }

    #[test]
    fn an_empty_binding_is_a_clear() {
        assert_eq!(validate_binding("speak", "", &[]), Ok(String::new()));
        assert_eq!(validate_binding("speak", "   ", &[]), Ok(String::new()));
    }
}
