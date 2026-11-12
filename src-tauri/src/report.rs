//! Problem report (FR-018/019, contracts/issue-report.md): a prefilled GitHub issue URL and the
//! diagnostics text for the clipboard.
//!
//! Everything is built here from constants the app owns: the webview never supplies the URL, and
//! translated, selected or clipboard text has no way in. The log tail is redacted (home directory)
//! and capped so the whole URL stays under 6,000 characters.

use std::path::Path;

use serde_json::{Map, Value};

use crate::core::settings::SettingsProfile;

/// Issue form the URL targets (authored in `docs/public/.github/ISSUE_TEMPLATE/`).
pub const ISSUE_TEMPLATE: &str = "bug_report.yml";
/// Endpoint for a new issue; the form is read from the public repository.
pub const ISSUE_URL: &str = "https://github.com/lbngoc/verso-translate/issues/new";
/// GitHub truncates long URLs; staying below this keeps the form prefill intact.
pub const MAX_URL_CHARS: usize = 6_000;
/// The clipboard copy carries the whole redacted log, bounded so it stays pasteable.
pub const MAX_DIAGNOSTICS_BYTES: usize = 200_000;

/// What the report may include about the app and the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportContext {
    pub version: String,
    /// `macos` or `windows` (`Appearance::os`).
    pub os: String,
    /// `Appearance::os_version`: macOS `14.4.1`, Windows `10.0.22631.4317`.
    pub os_version: String,
    pub arch: String,
    /// Ordered engine ids, as the pipeline uses them.
    pub engines: Vec<String>,
}

/// The dropdown label the issue form expects.
pub fn os_label(os: &str, os_version: &str, arch: &str) -> String {
    if os.eq_ignore_ascii_case("macos") {
        return if arch.contains("aarch64") || arch.contains("arm64") {
            "macOS (Apple Silicon)".into()
        } else {
            "macOS (Intel)".into()
        };
    }
    // "10.0.<build>.<ubr>": build 22000 and later is Windows 11.
    let build = os_version
        .split('.')
        .nth(2)
        .and_then(|part| part.parse::<u32>().ok())
        .unwrap_or(0);
    if build >= 22_000 {
        "Windows 11".into()
    } else {
        "Windows 10".into()
    }
}

/// Replaces the signed-in user's home directory with `~`, on both path styles.
pub fn redact_home(text: &str, home: Option<&str>) -> String {
    let Some(home) = home.map(str::trim).filter(|home| !home.is_empty()) else {
        return text.to_string();
    };
    text.replace(home, "~")
}

/// Keeps the newest part of a log (the tail), on a character boundary.
pub fn tail(text: &str, max_chars: usize) -> String {
    let total = text.chars().count();
    if total <= max_chars {
        return text.to_string();
    }
    text.chars().skip(total - max_chars).collect()
}

/// Percent-encodes a query value; unreserved characters stay as they are.
fn encode(value: &str) -> String {
    use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
    const QUERY: &AsciiSet = &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'~');
    utf8_percent_encode(value, QUERY).to_string()
}

/// Builds the prefilled issue URL, shrinking the log tail until the whole URL fits.
pub fn build_issue_url(context: &ReportContext, log_tail: &str, home: Option<&str>) -> String {
    let prefix = format!(
        "{ISSUE_URL}?template={ISSUE_TEMPLATE}&version={}&os={}&engines={}&logs=",
        encode(&context.version),
        encode(&os_label(&context.os, &context.os_version, &context.arch)),
        encode(&context.engines.join(", "))
    );
    let mut tail = redact_home(log_tail, home);
    loop {
        let url = format!("{prefix}{}", encode(&tail));
        if url.chars().count() <= MAX_URL_CHARS || tail.is_empty() {
            return url;
        }
        // Drop the oldest tenth: the newest lines explain the failure best.
        let total = tail.chars().count();
        let keep = total - total / 10 - 1;
        tail = tail.chars().skip(total - keep).collect();
    }
}

/// Settings without anything personal: a whitelist, so a field added later never leaks by default.
pub fn settings_summary(profile: &SettingsProfile) -> Value {
    let mut map = Map::new();
    map.insert("target_lang".into(), profile.target_lang.clone().into());
    map.insert(
        "secondary_lang".into(),
        profile.secondary_lang.clone().into(),
    );
    map.insert("source_lang".into(), profile.source_lang.clone().into());
    map.insert("engine_order".into(), profile.engine_order.clone().into());
    map.insert(
        "engines_disabled".into(),
        profile.engines_disabled.clone().into(),
    );
    map.insert("theme".into(), format!("{:?}", profile.theme).into());
    map.insert("ui_language".into(), profile.ui_language.clone().into());
    map.insert("launch_at_login".into(), profile.launch_at_login.into());
    map.insert("popup_expanded".into(), profile.popup_expanded.into());
    map.insert("check_updates".into(), profile.check_updates.into());
    Value::Object(map)
}

/// The clipboard text: version, system, settings without personal values, redacted log.
pub fn diagnostics(
    context: &ReportContext,
    profile: &SettingsProfile,
    log: &str,
    home: Option<&str>,
) -> String {
    let redacted = redact_home(log, home);
    let bounded = tail(&redacted, MAX_DIAGNOSTICS_BYTES);
    let settings =
        serde_json::to_string_pretty(&settings_summary(profile)).unwrap_or_else(|_| "{}".into());
    format!(
        "Verso {version}\n\
         OS: {os} ({os_version}, {arch})\n\
         Engines: {engines}\n\
         Settings:\n{settings}\n\n\
         Log (newest last):\n{bounded}\n",
        version = context.version,
        os = os_label(&context.os, &context.os_version, &context.arch),
        os_version = context.os_version,
        arch = context.arch,
        engines = context.engines.join(", "),
    )
}

/// Reads at most `max_bytes` from the end of the log file (newest lines), lossily decoded.
pub fn read_log_tail(path: &Path, max_bytes: usize) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(path) else {
        return String::new();
    };
    let size = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let start = size.saturating_sub(max_bytes as u64);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return String::new();
    }
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return String::new();
    }
    if start > 0 {
        // The first line is probably a fragment; drop it.
        if let Some(newline) = bytes.iter().position(|byte| *byte == b'\n') {
            bytes.drain(..=newline);
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> ReportContext {
        ReportContext {
            version: "0.1.0".into(),
            os: "macos".into(),
            os_version: "14.4.1".into(),
            arch: "aarch64".into(),
            engines: vec!["http-google".into(), "http-bing".into()],
        }
    }

    #[test]
    fn labels_match_the_issue_form_dropdown() {
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
    fn home_directory_is_redacted() {
        let text = "opened /Users/ngoc/Library/Logs/app.versotranslate/verso.log";
        assert_eq!(
            redact_home(text, Some("/Users/ngoc")),
            "opened ~/Library/Logs/app.versotranslate/verso.log"
        );
        assert_eq!(
            redact_home(r"C:\Users\ngoc\AppData\verso.log", Some(r"C:\Users\ngoc")),
            r"~\AppData\verso.log"
        );
    }

    #[test]
    fn tail_keeps_the_newest_lines() {
        assert_eq!(tail("abcdef", 3), "def");
        assert_eq!(tail("abc", 3), "abc");
        assert_eq!(tail("", 10), "");
    }

    #[test]
    fn url_carries_only_context_and_the_tail() {
        let url = build_issue_url(&context(), "line one\nline two", None);
        assert!(url.starts_with(ISSUE_URL));
        assert!(url.contains("template=bug_report.yml"));
        assert!(url.contains("version=0.1.0"));
        assert!(url.contains("os=macOS%20%28Apple%20Silicon%29"));
        assert!(url.contains("engines=http-google%2C%20http-bing"));
        assert!(url.contains("logs=line%20one%0Aline%20two"));
    }

    #[test]
    fn url_stays_under_the_cap_and_keeps_the_newest_log() {
        let log = format!("{}THE-END", "detail line\n".repeat(20_000));
        let url = build_issue_url(&context(), &log, None);
        assert!(url.chars().count() <= MAX_URL_CHARS);
        assert!(url.ends_with("THE-END"));
    }

    #[test]
    fn url_redacts_the_home_directory() {
        let url = build_issue_url(&context(), "/Users/ngoc/verso.log", Some("/Users/ngoc"));
        assert!(!url.contains("/Users/"));
        assert!(url.contains("~%2Fverso.log"));
    }

    #[test]
    fn diagnostics_omit_unknown_settings_fields() {
        let mut profile = SettingsProfile::default();
        profile
            .extra
            .insert("personal_note".into(), Value::String("secret".into()));
        let text = diagnostics(&context(), &profile, "log", None);
        assert!(text.contains("Verso 0.1.0"));
        assert!(text.contains("macOS (Apple Silicon)"));
        assert!(!text.contains("personal_note"));
        assert!(!text.contains("secret"));
    }

    #[test]
    fn diagnostics_are_bounded() {
        let big = "x".repeat(MAX_DIAGNOSTICS_BYTES * 2);
        let text = diagnostics(&context(), &SettingsProfile::default(), &big, None);
        assert!(text.len() < MAX_DIAGNOSTICS_BYTES + 1_000);
    }
}
