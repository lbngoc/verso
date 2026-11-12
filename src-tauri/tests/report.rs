//! Problem report contract tests (007 T049, contracts/issue-report.md).
//!
//! The field ids the URL prefills and the ids in the public issue form must not drift.

use std::path::PathBuf;

use verso_lib::report::{
    build_issue_url, diagnostics, os_label, redact_home, ReportContext, ISSUE_TEMPLATE,
    MAX_DIAGNOSTICS_BYTES, MAX_URL_CHARS,
};

fn form_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/public/.github/ISSUE_TEMPLATE")
        .join(ISSUE_TEMPLATE)
}

/// The `id:` values of the form's body items, in order.
fn form_field_ids() -> Vec<String> {
    let text = std::fs::read_to_string(form_path()).expect("the public bug form exists");
    let mut ids = Vec::new();
    for line in text.lines() {
        if let Some(id) = line.trim_start().strip_prefix("id:") {
            ids.push(id.trim().to_string());
        }
    }
    ids
}

fn context() -> ReportContext {
    ReportContext {
        version: "0.1.0".into(),
        os: "windows".into(),
        os_version: "10.0.22631.4317".into(),
        arch: "x86_64".into(),
        engines: vec!["http-google".into(), "http-bing".into()],
    }
}

#[test]
fn url_field_ids_exist_in_the_public_form() {
    let ids = form_field_ids();
    for required in ["version", "os", "engines", "steps", "expected", "logs"] {
        assert!(
            ids.iter().any(|id| id == required),
            "the public bug form has no '{required}' field (found {ids:?})"
        );
    }

    let url = build_issue_url(&context(), "log", None);
    for id in ["version", "os", "engines", "logs"] {
        assert!(
            url.contains(&format!("{id}=")),
            "the URL does not prefill '{id}'"
        );
    }
}

#[test]
fn url_uses_the_form_filename_and_a_bounded_log() {
    let marker = "MARKER-SENTENCE-THAT-MUST-NOT-LEAK";
    // Text the user translated is never passed in; only version, OS, engines and the log are.
    let log = format!("{}{}", "noise\n".repeat(30_000), marker);
    let url = build_issue_url(&context(), &log, None);

    assert!(url.contains(&format!("template={ISSUE_TEMPLATE}")));
    assert!(url.chars().count() <= MAX_URL_CHARS);
    assert!(url.ends_with(marker));
    assert!(url.contains("os=Windows%2011"));
}

#[test]
fn url_has_no_home_directory() {
    let url = build_issue_url(
        &context(),
        r"C:\Users\ngoc\AppData\Local\app.versotranslate\logs\verso.log",
        Some(r"C:\Users\ngoc"),
    );
    assert!(!url.contains("Users%5C"));
    assert!(url.contains("~%5CAppData"));
}

#[test]
fn labels_cover_both_platforms() {
    assert_eq!(
        os_label("macos", "14.4.1", "aarch64"),
        "macOS (Apple Silicon)"
    );
    assert_eq!(os_label("macos", "14.4.1", "x86_64"), "macOS (Intel)");
    assert_eq!(
        os_label("windows", "10.0.22631.4317", "x86_64"),
        "Windows 11"
    );
    assert_eq!(
        os_label("windows", "10.0.19045.3803", "x86_64"),
        "Windows 10"
    );
}

#[test]
fn diagnostics_hold_no_personal_values_and_stay_bounded() {
    let mut profile = verso_lib::core::settings::SettingsProfile::default();
    profile
        .extra
        .insert("owner_secret".into(), serde_json::json!("hunter2"));
    let log = "line with /Users/ngoc/secret\n".repeat(20_000);
    let text = diagnostics(&context(), &profile, &log, Some("/Users/ngoc"));

    assert!(text.contains("Verso 0.1.0"));
    assert!(text.contains("Windows 11"));
    assert!(!text.contains("owner_secret"));
    assert!(!text.contains("ngoc"));
    assert!(text.len() <= MAX_DIAGNOSTICS_BYTES + 1_000);
}

#[test]
fn redaction_keeps_other_paths() {
    assert_eq!(
        redact_home("/opt/homebrew/bin/verso", Some("/Users/ngoc")),
        "/opt/homebrew/bin/verso"
    );
}
