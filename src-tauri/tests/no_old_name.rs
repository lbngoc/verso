//! T008 — SC-004: no user-visible or project surface may still carry the old name.
//!
//! The needles are built at compile time so this guard does not flag itself, and are lowercased
//! against the file contents so the check is case-insensitive (the second needle covers the old
//! camel-case spelling).

use std::fs;
use std::path::{Path, PathBuf};

const NEEDLES: [&str; 2] = [concat!("meta", "trans"), concat!("m", "translate")];

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) {
    if path.is_file() {
        files.push(path.to_path_buf());
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("node_modules") {
                continue;
            }
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn is_html(path: &Path) -> bool {
    path.extension().and_then(|value| value.to_str()) == Some("html")
}

#[test]
fn the_old_name_is_gone_everywhere() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parent = root.parent().expect("src-tauri has a parent directory");

    let mut files = Vec::new();
    for relative in [
        "../src",
        "src",
        "tests",
        "examples",
        "tauri.conf.json",
        "Cargo.toml",
        "capabilities",
        "icons/source",
        "../package.json",
        "../scripts",
        "../THIRD_PARTY.md",
    ] {
        collect_files(&root.join(relative), &mut files);
    }
    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_html(&path) {
                files.push(path);
            }
        }
    }
    files.sort();
    files.dedup();
    let files: Vec<PathBuf> = files
        .into_iter()
        .map(|path| path.canonicalize().unwrap_or(path))
        .collect();
    assert!(!files.is_empty(), "the surfaces to sweep should exist");

    let mut hits = Vec::new();
    for file in files {
        let Ok(contents) = fs::read_to_string(&file) else {
            continue;
        };
        for (index, line) in contents.lines().enumerate() {
            let lower = line.to_lowercase();
            if NEEDLES.iter().any(|needle| lower.contains(needle)) {
                let shown = file.strip_prefix(parent).unwrap_or(&file);
                hits.push(format!(
                    "{}:{}: {}",
                    shown.display(),
                    index + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "the old name survives in {} place(s):\n{}",
        hits.len(),
        hits.join("\n")
    );
}
