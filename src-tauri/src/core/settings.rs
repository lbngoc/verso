//! Versioned JSON settings at `<app-data>/app.versotranslate/settings.json` (research P10).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::languages;

pub const SCHEMA_VERSION: u32 = 1;
pub const SETTINGS_FILE: &str = "settings.json";
pub const KNOWN_ENGINES: [&str; 2] = ["http-google", "http-bing"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

/// Where the accent colour comes from (FR-027/FR-028). Unknown values fall back to the app teal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    #[default]
    App,
    System,
}

impl<'de> Deserialize<'de> for Accent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // A hand-edited or newer value must not turn the whole file into a corrupt one.
        let raw = String::deserialize(deserializer)?;
        Ok(match raw.trim().to_ascii_lowercase().as_str() {
            "system" => Accent::System,
            _ => Accent::App,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shortcuts {
    pub translate: String,
    pub quick: String,
    /// Speak the selection (FR-009); empty means unbound.
    pub speak: String,
    /// Translate a screen region (spec 004). Registered only while the module is on.
    pub screen: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            translate: "Alt+Shift+Z".into(),
            quick: default_quick_shortcut().into(),
            speak: "Alt+Shift+S".into(),
            screen: default_screen_shortcut().into(),
        }
    }
}

/// The screen translate default (spec 004 R9): the same on both platforms.
pub fn default_screen_shortcut() -> &'static str {
    "Alt+Shift+X"
}

/// The screen translate module (spec 004): off until the user turns it on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OcrSettings {
    pub enabled: bool,
    /// `"auto"` or a BCP-47 code the system recognises.
    pub language: String,
}

impl Default for OcrSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            language: "auto".into(),
        }
    }
}

/// The quick-panel default (spec 009 Revision 2): the same on both platforms. It replaces
/// `Alt+Shift+Cmd+T` / `Ctrl+Alt+Shift+T`, which stay only for users who saved them.
pub fn default_quick_shortcut() -> &'static str {
    "Alt+Shift+T"
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsProfile {
    pub schema: u32,
    pub target_lang: String,
    pub secondary_lang: String,
    pub source_lang: String,
    pub engine_order: Vec<String>,
    pub engines_disabled: Vec<String>,
    pub shortcuts: Shortcuts,
    pub ocr: OcrSettings,
    pub theme: Theme,
    pub launch_at_login: bool,
    pub ui_language: String,
    pub popup_expanded: bool,
    pub first_run_engine_choice_at: Option<String>,
    /// The version that ran last (T059): a change means the app was updated, and macOS may need the
    /// Accessibility grant again for a new code identity.
    pub last_run_version: String,
    /// Automatic update checks (FR-016): chosen during onboarding, changeable in Settings.
    pub check_updates: bool,
    pub telemetry_opt_in: bool,
    /// Paste the clipboard into the quick panel when nothing is selected (FR-011). Off by default.
    pub quick_paste_clipboard: bool,
    /// Accent colour source (FR-027): the app teal, or the system accent behind a contrast guard.
    pub accent: Accent,
    /// Result text size step, clamped to −2…=4 (FR-005).
    pub text_size_step: i8,
    /// Version the user skipped in the update dialog (FR-023); must parse as a version.
    pub skipped_update_version: Option<String>,
    /// One-time notes already seen by the user (R7), e.g. `speak-unbound`.
    pub notes_seen: Vec<String>,
    /// Unknown fields written by a newer build are preserved on save.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

impl Default for SettingsProfile {
    fn default() -> Self {
        Self {
            schema: SCHEMA_VERSION,
            target_lang: "vi".into(),
            secondary_lang: "en".into(),
            source_lang: "auto".into(),
            engine_order: KNOWN_ENGINES.iter().map(|id| id.to_string()).collect(),
            engines_disabled: Vec::new(),
            shortcuts: Shortcuts::default(),
            ocr: OcrSettings::default(),
            theme: Theme::System,
            launch_at_login: false,
            ui_language: default_ui_language(),
            popup_expanded: false,
            first_run_engine_choice_at: None,
            last_run_version: String::new(),
            check_updates: false,
            telemetry_opt_in: false,
            quick_paste_clipboard: false,
            accent: Accent::App,
            text_size_step: 0,
            skipped_update_version: None,
            notes_seen: Vec::new(),
            extra: serde_json::Map::new(),
        }
    }
}

pub fn default_ui_language() -> String {
    let locale = sys_locale::get_locale().unwrap_or_else(|| "en".into());
    if locale.to_ascii_lowercase().starts_with("vi") {
        "vi".into()
    } else {
        "en".into()
    }
}

/// Accepts `x.y.z` (an optional leading `v` is dropped); anything else is not a version.
pub fn parse_version(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let without_v = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    let parts: Vec<&str> = without_v.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let numeric = parts
        .iter()
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    numeric.then(|| without_v.to_string())
}

impl SettingsProfile {
    /// Clamps a profile into a state the app can honour (FR-016: never every engine off).
    pub fn validated(mut self) -> Self {
        self.schema = SCHEMA_VERSION;
        self.target_lang = languages::normalise(&self.target_lang);
        if self.target_lang == "auto" {
            self.target_lang = "vi".into();
        }
        self.secondary_lang = languages::normalise(&self.secondary_lang);
        // FR-014: the secondary language must actually differ from the target.
        if self.secondary_lang == self.target_lang {
            self.secondary_lang = if self.target_lang == "en" {
                "vi".into()
            } else {
                "en".into()
            };
        }
        self.source_lang = languages::normalise(&self.source_lang);
        if self.source_lang.is_empty() {
            self.source_lang = "auto".into();
        }

        // Engine order: known ids first, in the user's order, then anything missing.
        let mut order: Vec<String> = Vec::new();
        for id in self
            .engine_order
            .iter()
            .map(String::as_str)
            .chain(KNOWN_ENGINES.iter().copied())
        {
            let id = id.trim();
            if KNOWN_ENGINES.contains(&id) && !order.iter().any(|known| known == id) {
                order.push(id.to_string());
            }
        }
        self.engine_order = order;

        let mut disabled: Vec<String> = Vec::new();
        for id in &self.engines_disabled {
            let id = id.trim();
            if KNOWN_ENGINES.contains(&id) && !disabled.iter().any(|known| known == id) {
                disabled.push(id.to_string());
            }
        }
        if disabled.len() >= self.engine_order.len() {
            disabled.retain(|id| id != &self.engine_order[0]);
        }
        self.engines_disabled = disabled;

        if self.shortcuts.translate.trim().is_empty() {
            self.shortcuts.translate.clear();
        }
        if self.shortcuts.quick.trim().is_empty() {
            self.shortcuts.quick.clear();
        }
        self.shortcuts.translate = self.shortcuts.translate.trim().to_string();
        self.shortcuts.quick = self.shortcuts.quick.trim().to_string();
        self.shortcuts.speak = self.shortcuts.speak.trim().to_string();
        // Revision 2: a `quick` missing from an old file takes the new default, which may equal a
        // binding the user saved (their old Translate `Alt+Shift+T`). Saved values win: leave the
        // new default unbound; Settings shows the one-time note (`quick-unbound`).
        if !self.shortcuts.quick.is_empty()
            && (self.shortcuts.quick == self.shortcuts.translate
                || self.shortcuts.quick == self.shortcuts.speak)
        {
            self.shortcuts.quick.clear();
        }
        // R7: an added `speak` default may collide with a binding the user already had. Leave it
        // unbound; Settings shows the one-time note (the UI records `speak-unbound` when dismissed).
        if !self.shortcuts.speak.is_empty()
            && (self.shortcuts.speak == self.shortcuts.translate
                || self.shortcuts.speak == self.shortcuts.quick)
        {
            self.shortcuts.speak.clear();
        }
        // Spec 004: the screen shortcut follows the same rule; saved bindings win over the default.
        self.shortcuts.screen = self.shortcuts.screen.trim().to_string();
        if !self.shortcuts.screen.is_empty()
            && (self.shortcuts.screen == self.shortcuts.translate
                || self.shortcuts.screen == self.shortcuts.speak
                || self.shortcuts.screen == self.shortcuts.quick)
        {
            self.shortcuts.screen.clear();
        }
        self.ocr.language = self.ocr.language.trim().to_string();
        if self.ocr.language.is_empty() {
            self.ocr.language = "auto".into();
        }
        self.text_size_step = self.text_size_step.clamp(-2, 4);
        self.skipped_update_version = self
            .skipped_update_version
            .take()
            .and_then(|version| parse_version(&version));
        if self.ui_language != "vi" {
            self.ui_language = "en".into();
        }
        if self.telemetry_opt_in {
            // 002 has no telemetry at all; a stray value from another build is not honoured.
            self.telemetry_opt_in = false;
        }
        self
    }

    pub fn engine_enabled(&self, id: &str) -> bool {
        !self.engines_disabled.iter().any(|disabled| disabled == id)
    }
}

/// Whether the app should guide the user to grant Accessibility again after an update (T059).
///
/// No first run (onboarding handles it) and no prompt while the grant is intact. A stored version that
/// differs from the running one is an update; an empty one is an upgrade from a build that never recorded
/// it (0.1.0 and 0.1.1), which had no permission baseline either.
pub fn needs_permission_regrant(
    onboarded: bool,
    last_run_version: &str,
    current_version: &str,
    permission_granted: bool,
) -> bool {
    onboarded && last_run_version != current_version && !permission_granted
}

/// The reason the permission step opens with: "after an update" only when the startup check asked
/// for a re-grant (spec 010 FR-003, FR-007).
pub fn permission_reason(regrant: bool) -> crate::platform::PermissionReason {
    if regrant {
        crate::platform::PermissionReason::AfterUpdate
    } else {
        crate::platform::PermissionReason::Missing
    }
}

/// The service name `tccutil` and TCC use for a permission kind (spec 010 R2).
pub fn tcc_service(kind: crate::platform::PermissionKind) -> &'static str {
    match kind {
        crate::platform::PermissionKind::Accessibility => "Accessibility",
        crate::platform::PermissionKind::ScreenRecording => "ScreenCapture",
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("could not read settings: {0}")]
    Read(#[source] std::io::Error),
    #[error("could not write settings: {0}")]
    Write(#[source] std::io::Error),
    #[error("settings patch is invalid: {0}")]
    InvalidPatch(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWarning {
    /// The file could not be parsed; it was renamed to this backup and defaults were used.
    Corrupt { backup: PathBuf },
}

/// Reads a profile, moving a corrupt file aside instead of failing the app (never blocks startup).
pub fn load_profile(path: &Path) -> (SettingsProfile, Option<LoadWarning>) {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<SettingsProfile>(&text) {
            Ok(profile) => (profile.validated(), None),
            Err(_) => match backup_corrupt(path) {
                Some(backup) => (
                    SettingsProfile::default(),
                    Some(LoadWarning::Corrupt { backup }),
                ),
                None => (SettingsProfile::default(), None),
            },
        },
        Err(_) => (SettingsProfile::default(), None),
    }
}

fn backup_corrupt(path: &Path) -> Option<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let backup = path.with_file_name(format!("settings.corrupt-{stamp}.json"));
    fs::rename(path, &backup).ok().map(|_| backup)
}

/// Writes JSON atomically: temp file in the same directory, fsync, then rename over the target.
pub fn write_profile_atomic(path: &Path, profile: &SettingsProfile) -> Result<(), SettingsError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(SettingsError::Write)?;
    }
    let json = serde_json::to_vec_pretty(profile).map_err(|error| {
        SettingsError::InvalidPatch(format!("settings could not be serialised: {error}"))
    })?;
    let temp = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&temp).map_err(SettingsError::Write)?;
        file.write_all(&json).map_err(SettingsError::Write)?;
        file.sync_all().map_err(SettingsError::Write)?;
    }
    fs::rename(&temp, path).map_err(SettingsError::Write)
}

/// The live store every window reads; writes are atomic and unknown fields survive.
#[derive(Debug)]
pub struct SettingsStore {
    path: PathBuf,
    current: RwLock<SettingsProfile>,
    pub warning: Option<LoadWarning>,
}

impl SettingsStore {
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let (profile, warning) = load_profile(&path);
        Self {
            path,
            current: RwLock::new(profile),
            warning,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self) -> SettingsProfile {
        self.current.read().clone()
    }

    /// Applies a partial JSON patch (as the UI sends it), validates, then writes through.
    pub fn update(&self, patch: &Value) -> Result<SettingsProfile, SettingsError> {
        let object = patch
            .as_object()
            .ok_or_else(|| SettingsError::InvalidPatch("expected an object".into()))?;
        let mut value = serde_json::to_value(self.get()).map_err(|error| {
            SettingsError::InvalidPatch(format!("current settings are not serialisable: {error}"))
        })?;
        // An empty recognition language is refused rather than quietly turned into "auto".
        if let Some(language) = object
            .get("ocr")
            .and_then(|ocr| ocr.get("language"))
            .and_then(Value::as_str)
        {
            if language.trim().is_empty() {
                return Err(SettingsError::InvalidPatch(
                    "ocr.language must be \"auto\" or a language code".into(),
                ));
            }
        }
        merge_json(&mut value, &Value::Object(object.clone()));
        let parsed: SettingsProfile = serde_json::from_value(value)
            .map_err(|error| SettingsError::InvalidPatch(error.to_string()))?;
        let profile = parsed.validated();
        write_profile_atomic(&self.path, &profile)?;
        *self.current.write() = profile.clone();
        Ok(profile)
    }

    pub fn replace(&self, profile: SettingsProfile) -> Result<SettingsProfile, SettingsError> {
        let profile = profile.validated();
        write_profile_atomic(&self.path, &profile)?;
        *self.current.write() = profile.clone();
        Ok(profile)
    }

    /// Re-reads the file (used after a second instance changes settings).
    pub fn reload(&self) {
        let (profile, _) = load_profile(&self.path);
        *self.current.write() = profile;
    }
}

/// Recursive object merge: `null` deletes a key, arrays replace wholesale.
fn merge_json(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, value) in patch {
                if value.is_null() {
                    target.remove(key);
                } else {
                    merge_json(target.entry(key.clone()).or_insert(Value::Null), value);
                }
            }
        }
        (target, patch) => {
            if !patch.is_null() {
                *target = patch.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_order_and_schema() {
        let profile = SettingsProfile::default();
        assert_eq!(profile.schema, 1);
        assert_eq!(profile.engine_order, vec!["http-google", "http-bing"]);
        assert!(profile.engines_disabled.is_empty());
        assert_eq!(profile.shortcuts.translate, "Alt+Shift+Z");
        assert_eq!(profile.shortcuts.speak, "Alt+Shift+S");
        assert_eq!(profile.shortcuts.quick, "Alt+Shift+T");
    }

    #[test]
    fn validation_never_disables_every_engine() {
        let profile = SettingsProfile {
            engines_disabled: vec!["http-google".into(), "http-bing".into()],
            ..SettingsProfile::default()
        }
        .validated();
        assert_eq!(profile.engines_disabled, vec!["http-bing"]);
    }

    #[test]
    fn merge_preserves_unknown_fields() {
        let mut target = serde_json::json!({"a": 1, "nested": {"x": 2, "y": 3}});
        merge_json(
            &mut target,
            &serde_json::json!({"b": 2, "nested": {"y": 4}}),
        );
        assert_eq!(
            target,
            serde_json::json!({"a": 1, "b": 2, "nested": {"x": 2, "y": 4}})
        );
    }

    #[test]
    fn regrant_step_only_after_an_update_without_the_grant() {
        assert!(needs_permission_regrant(true, "0.1.0", "0.1.1", false));
        // Not until onboarding has ended.
        assert!(!needs_permission_regrant(false, "0.1.0", "0.1.1", false));
        // No update (same version): nothing to do.
        assert!(!needs_permission_regrant(true, "0.1.1", "0.1.1", false));
        // An empty baseline is an upgrade from a build that never stored one (0.1.0, 0.1.1).
        assert!(needs_permission_regrant(true, "", "0.1.3", false));
        // The grant survived: nothing to do.
        assert!(!needs_permission_regrant(true, "0.1.0", "0.1.1", true));
        assert!(!needs_permission_regrant(true, "", "0.1.3", true));
    }

    #[test]
    fn permission_reason_is_after_update_only_for_a_regrant() {
        use crate::platform::PermissionReason;
        assert_eq!(permission_reason(true), PermissionReason::AfterUpdate);
        assert_eq!(permission_reason(false), PermissionReason::Missing);
        assert_eq!(PermissionReason::default(), PermissionReason::Missing);
    }

    #[test]
    fn tcc_service_names_match_tccutil() {
        use crate::platform::PermissionKind;
        assert_eq!(tcc_service(PermissionKind::Accessibility), "Accessibility");
        assert_eq!(
            tcc_service(PermissionKind::ScreenRecording),
            "ScreenCapture"
        );
    }

    fn load_json(text: &str) -> (SettingsProfile, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE);
        fs::write(&path, text).expect("write settings");
        let (profile, warning) = load_profile(&path);
        assert!(warning.is_none(), "a valid file must not warn");
        (profile, dir)
    }

    #[test]
    fn a_profile_saved_with_the_old_shortcuts_is_kept_unchanged() {
        let (profile, _dir) = load_json(
            r#"{"shortcuts":{"translate":"Alt+Shift+T","quick":"Alt+Shift+Cmd+T","speak":"Alt+Shift+S"}}"#,
        );
        assert_eq!(profile.shortcuts.translate, "Alt+Shift+T");
        assert_eq!(profile.shortcuts.quick, "Alt+Shift+Cmd+T");
        assert_eq!(profile.shortcuts.speak, "Alt+Shift+S");
    }

    #[test]
    fn a_file_without_speak_gains_its_default() {
        let (profile, _dir) = load_json(r#"{"shortcuts":{"translate":"Alt+Shift+Z"}}"#);
        assert_eq!(profile.shortcuts.speak, "Alt+Shift+S");
        assert_eq!(profile.shortcuts.translate, "Alt+Shift+Z");
    }

    #[test]
    fn a_missing_quick_whose_default_collides_is_left_unbound_without_a_note_yet() {
        let (profile, _dir) = load_json(r#"{"shortcuts":{"translate":"Alt+Shift+T"}}"#);
        assert_eq!(profile.shortcuts.translate, "Alt+Shift+T");
        assert_eq!(profile.shortcuts.quick, "");
        assert!(!profile
            .notes_seen
            .iter()
            .any(|note| note == "quick-unbound"));
    }

    #[test]
    fn a_missing_quick_without_a_collision_gains_its_default() {
        let (profile, _dir) = load_json(r#"{"shortcuts":{"translate":"Alt+Shift+Y"}}"#);
        assert_eq!(profile.shortcuts.quick, "Alt+Shift+T");
    }

    #[test]
    fn a_colliding_speak_binding_is_left_unbound_without_a_note_yet() {
        let (profile, _dir) = load_json(r#"{"shortcuts":{"translate":"Alt+Shift+S"}}"#);
        assert_eq!(profile.shortcuts.speak, "");
        // The note is queued for the UI, not silently consumed by the migration.
        assert!(!profile
            .notes_seen
            .iter()
            .any(|note| note == "speak-unbound"));
    }

    #[test]
    fn an_empty_binding_means_unbound() {
        let (profile, _dir) = load_json(r#"{"shortcuts":{"translate":"","quick":"","speak":""}}"#);
        assert_eq!(profile.shortcuts.translate, "");
        assert_eq!(profile.shortcuts.quick, "");
        assert_eq!(profile.shortcuts.speak, "");
    }

    #[test]
    fn text_size_step_is_clamped_and_accent_falls_back() {
        let (profile, _dir) = load_json(r#"{"text_size_step":9,"accent":"magenta"}"#);
        assert_eq!(profile.text_size_step, 4);
        assert_eq!(profile.accent, Accent::App);
        let (profile, _dir) = load_json(r#"{"text_size_step":-7}"#);
        assert_eq!(profile.text_size_step, -2);
        let (profile, _dir) = load_json(r#"{"accent":"system"}"#);
        assert_eq!(profile.accent, Accent::System);
    }

    #[test]
    fn an_unparsable_skipped_version_is_dropped() {
        let (profile, _dir) = load_json(r#"{"skipped_update_version":"next"}"#);
        assert_eq!(profile.skipped_update_version, None);
        let (profile, _dir) = load_json(r#"{"skipped_update_version":"0.2.0"}"#);
        assert_eq!(profile.skipped_update_version.as_deref(), Some("0.2.0"));
    }

    #[test]
    fn new_options_default_to_off() {
        let profile = SettingsProfile::default();
        assert!(!profile.quick_paste_clipboard);
        assert_eq!(profile.text_size_step, 0);
        assert_eq!(profile.accent, Accent::App);
        assert!(profile.notes_seen.is_empty());
        assert_eq!(profile.skipped_update_version, None);
    }

    #[test]
    fn unknown_fields_and_popup_expanded_survive_a_save() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE);
        fs::write(
            &path,
            r#"{"popup_expanded":true,"future_field":{"nested":1}}"#,
        )
        .expect("write settings");
        let store = SettingsStore::open(&path);
        store
            .update(&serde_json::json!({"text_size_step": 2}))
            .expect("update");
        let text = fs::read_to_string(&path).expect("read");
        let value: Value = serde_json::from_str(&text).expect("json");
        assert_eq!(value["popup_expanded"], Value::Bool(true));
        assert_eq!(value["future_field"]["nested"], Value::from(1));
        assert_eq!(value["text_size_step"], Value::from(2));
    }
}
