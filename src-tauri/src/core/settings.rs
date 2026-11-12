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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shortcuts {
    pub translate: String,
    pub quick: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            translate: "Alt+Shift+T".into(),
            quick: default_quick_shortcut().into(),
        }
    }
}

pub fn default_quick_shortcut() -> &'static str {
    if cfg!(target_os = "macos") {
        "Alt+Shift+Cmd+T"
    } else {
        "Ctrl+Alt+Shift+T"
    }
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
            theme: Theme::System,
            launch_at_login: false,
            ui_language: default_ui_language(),
            popup_expanded: false,
            first_run_engine_choice_at: None,
            last_run_version: String::new(),
            check_updates: false,
            telemetry_opt_in: false,
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
            self.shortcuts.translate = Shortcuts::default().translate;
        }
        if self.shortcuts.quick.trim().is_empty() {
            self.shortcuts.quick = Shortcuts::default().quick;
        }
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
/// Only a real version change counts: no first run (onboarding handles it), no prompt while the grant
/// is intact, and nothing when the stored version is still empty (no baseline yet).
pub fn needs_permission_regrant(
    onboarded: bool,
    last_run_version: &str,
    current_version: &str,
    permission_granted: bool,
) -> bool {
    onboarded
        && !last_run_version.is_empty()
        && last_run_version != current_version
        && !permission_granted
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
        assert_eq!(profile.shortcuts.translate, "Alt+Shift+T");
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
        // No update (same version), or no baseline yet.
        assert!(!needs_permission_regrant(true, "0.1.1", "0.1.1", false));
        assert!(!needs_permission_regrant(true, "", "0.1.1", false));
        // The grant survived: nothing to do.
        assert!(!needs_permission_regrant(true, "0.1.0", "0.1.1", true));
    }
}
