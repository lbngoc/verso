//! T016 — settings defaults, validation, atomic writes and corrupt-file recovery.

mod common;

use std::fs;

use serde_json::json;

use verso_lib::core::settings::{
    load_profile, SettingsProfile, SettingsStore, Theme, KNOWN_ENGINES, SCHEMA_VERSION,
};

#[test]
fn defaults_match_the_data_model() {
    let profile = SettingsProfile::default();
    assert_eq!(profile.schema, SCHEMA_VERSION);
    assert_eq!(
        profile.engine_order,
        vec!["http-google".to_string(), "http-bing".to_string()]
    );
    assert!(profile.engines_disabled.is_empty());
    assert_eq!(profile.target_lang, "vi");
    assert_eq!(profile.secondary_lang, "en");
    assert_eq!(profile.source_lang, "auto");
    assert_eq!(profile.shortcuts.translate, "Alt+Shift+Z");
    assert_eq!(profile.shortcuts.quick, "Alt+Shift+T");
    assert_eq!(profile.shortcuts.speak, "Alt+Shift+S");
    assert!(matches!(profile.theme, Theme::System));
    assert!(profile.first_run_engine_choice_at.is_none());
    assert!(!profile.telemetry_opt_in);
}

#[test]
fn store_writes_and_reads_back() {
    let (_dir, store) = common::temp_store();
    let updated = store
        .update(&json!({ "target_lang": "ja", "theme": "dark" }))
        .expect("update");
    assert_eq!(updated.target_lang, "ja");
    assert!(matches!(updated.theme, Theme::Dark));

    let reopened = SettingsStore::open(store.path());
    assert_eq!(reopened.get().target_lang, "ja");
}

#[test]
fn unknown_fields_are_preserved() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let mut profile = SettingsProfile::default();
    profile
        .extra
        .insert("future_option".into(), json!({ "nested": true }));
    verso_lib::core::settings::write_profile_atomic(&path, &profile).unwrap();

    let store = SettingsStore::open(&path);
    let updated = store
        .update(&json!({ "target_lang": "fr" }))
        .expect("update");
    assert_eq!(
        updated.extra.get("future_option"),
        Some(&json!({ "nested": true }))
    );

    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("future_option"));
}

#[test]
fn atomic_write_leaves_no_temp_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let profile = SettingsProfile::default();
    verso_lib::core::settings::write_profile_atomic(&path, &profile).unwrap();
    assert!(path.exists());
    assert!(!directory.path().join("settings.json.tmp").exists());
    let parsed: SettingsProfile =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(parsed.schema, SCHEMA_VERSION);
}

#[test]
fn corrupt_file_falls_back_to_defaults_and_keeps_a_backup() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    fs::write(&path, "{ this is not json").unwrap();

    let (profile, warning) = load_profile(&path);
    assert_eq!(profile.target_lang, "vi");
    let backup = match warning {
        Some(verso_lib::core::settings::LoadWarning::Corrupt { backup }) => backup,
        None => panic!("expected a corrupt-file warning"),
    };
    assert!(backup.exists(), "corrupt settings must be kept as a backup");
    assert!(!path.exists(), "the broken file is moved aside");
}

#[test]
fn validation_keeps_at_least_one_engine_enabled() {
    let profile = SettingsProfile {
        engines_disabled: vec!["http-google".into(), "http-bing".into()],
        ..SettingsProfile::default()
    }
    .validated();
    assert_eq!(profile.engines_disabled.len(), 1);
    assert!(profile
        .engine_order
        .iter()
        .any(|id| !profile.engines_disabled.contains(id)));
    assert!(profile
        .engine_order
        .iter()
        .all(|id| KNOWN_ENGINES.contains(&id.as_str())));
}

#[test]
fn validation_normalises_engine_order() {
    let profile = SettingsProfile {
        engine_order: vec!["bogus".into(), "http-bing".into(), "http-bing".into()],
        ..SettingsProfile::default()
    }
    .validated();
    assert_eq!(
        profile.engine_order,
        vec!["http-bing".to_string(), "http-google".to_string()]
    );
}

#[test]
fn secondary_language_differs_from_target() {
    let profile = SettingsProfile {
        target_lang: "en".into(),
        secondary_lang: "en".into(),
        ..SettingsProfile::default()
    }
    .validated();
    assert_eq!(profile.secondary_lang, "vi");

    let profile = SettingsProfile {
        target_lang: "vi".into(),
        secondary_lang: "vi".into(),
        ..SettingsProfile::default()
    }
    .validated();
    assert_eq!(profile.secondary_lang, "en");
}

#[test]
fn telemetry_is_always_off() {
    let (_dir, store) = common::temp_store();
    let profile = store
        .update(&json!({ "telemetry_opt_in": true }))
        .expect("update");
    assert!(!profile.telemetry_opt_in);
}

// ---- spec 004: the screen translate module (T004) ----

#[test]
fn a_fresh_profile_has_the_module_off_and_the_default_shortcut() {
    let profile = SettingsProfile::default();
    assert!(!profile.ocr.enabled);
    assert_eq!(profile.ocr.language, "auto");
    assert_eq!(profile.shortcuts.screen, "Alt+Shift+X");
}

#[test]
fn a_file_without_the_ocr_fields_loads_their_defaults() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        r#"{"schema":1,"target_lang":"ja","shortcuts":{"translate":"Alt+Shift+Z","speak":"Alt+Shift+S","quick":"Alt+Shift+T"}}"#,
    )
    .unwrap();
    let (profile, warning) = load_profile(&path);
    assert!(warning.is_none());
    assert_eq!(profile.schema, 1);
    assert!(!profile.ocr.enabled);
    assert_eq!(profile.ocr.language, "auto");
    assert_eq!(profile.shortcuts.screen, "Alt+Shift+X");
    assert_eq!(profile.target_lang, "ja");
}

#[test]
fn an_ocr_patch_round_trips_through_save_and_load() {
    let (_dir, store) = common::temp_store();
    store
        .update(&json!({ "ocr": { "enabled": true, "language": "fr" } }))
        .expect("update");
    let reopened = SettingsStore::open(store.path());
    let profile = reopened.get();
    assert!(profile.ocr.enabled);
    assert_eq!(profile.ocr.language, "fr");
    // A partial patch keeps the other field.
    reopened
        .update(&json!({ "ocr": { "enabled": false } }))
        .unwrap();
    assert_eq!(reopened.get().ocr.language, "fr");
}

#[test]
fn the_recognition_language_is_auto_or_a_non_empty_code() {
    let (_dir, store) = common::temp_store();
    assert!(store.update(&json!({ "ocr": { "language": "" } })).is_err());
    assert!(store
        .update(&json!({ "ocr": { "language": "   " } }))
        .is_err());
    assert_eq!(
        store.get().ocr.language,
        "auto",
        "a refused patch changes nothing"
    );
    // A well-formed code the system does not know is kept as written (it falls back at capture time).
    store
        .update(&json!({ "ocr": { "language": "xx-YY" } }))
        .expect("well-formed codes are kept");
    assert_eq!(store.get().ocr.language, "xx-YY");
    store
        .update(&json!({ "ocr": { "language": "auto" } }))
        .unwrap();
    assert_eq!(store.get().ocr.language, "auto");
}

#[test]
fn the_screen_shortcut_cannot_share_another_binding() {
    let profile = SettingsProfile {
        shortcuts: verso_lib::core::settings::Shortcuts {
            screen: "Alt+Shift+Z".into(),
            ..Default::default()
        },
        ..SettingsProfile::default()
    }
    .validated();
    assert_eq!(
        profile.shortcuts.screen, "",
        "left unbound, like quick and speak"
    );
    assert_eq!(profile.shortcuts.translate, "Alt+Shift+Z");
}
