//! Logging policy (FR-012, constitution V).
//!
//! Translated/selected text, clipboard contents and request URLs (which carry the text) are only
//! ever logged at debug level, and URLs are redacted even then. At info and above only engine ids,
//! counts, durations and error kinds are written.
//!
//! The app writes to the OS log directory through `tauri-plugin-log`
//! (`~/Library/Logs/app.versotranslate` on macOS, `%LOCALAPPDATA%\app.versotranslate\logs` on Windows).

use std::time::Instant;

use log::debug;
use tauri::plugin::TauriPlugin;
use tauri::Runtime;
use tauri_plugin_log::{Target, TargetKind};

/// Logs how long a user-visible step took, without any text or URL.
pub fn debug_duration(step: &str, started: Instant) {
    debug!("{step} took {} ms", started.elapsed().as_millis());
}

/// Keeps at most `max` characters; used before a debug-level excerpt.
pub fn excerpt(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        out.push('…');
    }
    out
}

/// Drops the query string and fragment (the text travels there on gtx).
pub fn redact_url(url: &str) -> String {
    let without_fragment = url.split('#').next().unwrap_or(url);
    match without_fragment.split_once('?') {
        Some((base, _)) => format!("{base}?…"),
        None => without_fragment.to_string(),
    }
}

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_log::Builder::new()
        .targets([
            Target::new(TargetKind::LogDir {
                file_name: Some("verso".into()),
            }),
            Target::new(TargetKind::Stdout),
        ])
        .level(log::LevelFilter::Info)
        .max_file_size(2_000_000)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_redacted() {
        let url = "https://translate.googleapis.com/translate_a/single?client=gtx&q=secret";
        assert_eq!(
            redact_url(url),
            "https://translate.googleapis.com/translate_a/single?…"
        );
        assert_eq!(
            redact_url("https://example.com/path"),
            "https://example.com/path"
        );
    }

    #[test]
    fn excerpts_are_capped() {
        assert_eq!(excerpt("abcdef", 3), "abc…");
        assert_eq!(excerpt("abc", 3), "abc");
    }
}
