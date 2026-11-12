//! T066 — no `info!`/`warn!`/`error!` call may format the selected/translated text or a request URL.

use std::fs;
use std::path::{Path, PathBuf};

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

#[test]
fn logging_above_debug_never_contains_text_or_urls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "the source tree should contain Rust files"
    );

    let forbidden = [
        ".text",
        "request.text",
        "translated",
        "selected_text",
        "q=",
        "redact_url",
        "clipboard text",
        "url =",
    ];

    let mut violations = Vec::new();
    for file in files {
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };
        for (index, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            let forbidden_macro = trimmed.starts_with("info!")
                || trimmed.starts_with("warn!")
                || trimmed.starts_with("error!");
            if forbidden_macro {
                for needle in forbidden {
                    if line.contains(needle) {
                        violations.push(format!("{}:{}: {}", file.display(), index + 1, trimmed));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "text or URLs are logged above debug level:\n{}",
        violations.join("\n")
    );
}

#[test]
fn the_redaction_helper_takes_the_query_string_away() {
    assert_eq!(
        verso_lib::core::logging::redact_url("https://example.com/a?q=secret"),
        "https://example.com/a?…"
    );
}
