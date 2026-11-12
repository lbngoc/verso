//! The locale table generated from translate-shell's `LanguageData.awk`.
//!
//! `resources/languages.json` is committed, so the app never reads the reference checkout at
//! runtime. The table maps our BCP-47 codes to each service's own codes (engine-contract § http-bing).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const RAW: &str = include_str!("../../resources/languages.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    pub code: String,
    pub name_en: String,
    pub name_native: String,
    pub rtl: bool,
    pub google: Option<String>,
    pub bing: Option<String>,
}

static LANGUAGES: OnceLock<Vec<Language>> = OnceLock::new();

pub fn all() -> &'static [Language] {
    LANGUAGES.get_or_init(|| {
        serde_json::from_str(RAW).expect("resources/languages.json is generated and valid")
    })
}

pub fn lookup(code: &str) -> Option<&'static Language> {
    all().iter().find(|language| language.code == code)
}

/// Normalises aliases the app accepts from the OS or older settings (`zh` → `zh-CN`).
pub fn normalise(code: &str) -> String {
    let code = code.trim();
    let lowered = code.to_ascii_lowercase();
    match lowered.as_str() {
        "zh" | "zh-hans" | "zh-chs" | "zh-cn" => "zh-CN".into(),
        "zh-hant" | "zh-cht" | "zh-tw" => "zh-TW".into(),
        "pt-br" => "pt-BR".into(),
        "no" | "nb" => "no".into(),
        _ => code.to_string(),
    }
}

/// Google's own code for a locale (falls back to the locale itself).
pub fn google_code(code: &str) -> String {
    let code = normalise(code);
    lookup(&code)
        .and_then(|language| language.google.clone())
        .unwrap_or(code)
}

/// Microsoft/Bing's own code for a locale (`zh-CN` → `zh-Hans`, `pt-PT` → `pt-pt`, …).
pub fn bing_code(code: &str) -> String {
    let code = normalise(code);
    if let Some(language) = lookup(&code) {
        if let Some(bing) = &language.bing {
            return bing.clone();
        }
    }
    // Hot patches ported from BingTranslator.awk for codes outside the locale table.
    match code.as_str() {
        "auto" => "auto-detect".into(),
        "tl" => "fil".into(),
        "hmn" => "mww".into(),
        "ku" => "kmr".into(),
        "ckb" => "ku".into(),
        "mn" => "mn-Cyrl".into(),
        "no" => "nb".into(),
        "zh-CN" => "zh-Hans".into(),
        "zh-TW" => "zh-Hant".into(),
        "pt-BR" => "pt".into(),
        "pt-PT" => "pt-pt".into(),
        _ => code,
    }
}

pub fn is_supported_by_google(code: &str) -> bool {
    lookup(&normalise(code))
        .map(|language| language.google.is_some())
        .unwrap_or(false)
}

pub fn is_supported_by_bing(code: &str) -> bool {
    lookup(&normalise(code))
        .map(|language| language.bing.is_some())
        .unwrap_or(false)
}

pub fn is_rtl(code: &str) -> bool {
    lookup(&normalise(code))
        .map(|language| language.rtl)
        .unwrap_or(false)
}

pub fn display_name(code: &str) -> String {
    lookup(&normalise(code))
        .map(|language| language.name_en.clone())
        .unwrap_or_else(|| code.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_the_mvp_languages() {
        for code in [
            "en", "vi", "zh-CN", "zh-TW", "ja", "ko", "ru", "ar", "pt-PT",
        ] {
            assert!(lookup(code).is_some(), "{code} should be in the table");
        }
    }

    #[test]
    fn bing_codes_follow_the_contract() {
        assert_eq!(bing_code("zh-CN"), "zh-Hans");
        assert_eq!(bing_code("zh-TW"), "zh-Hant");
        assert_eq!(bing_code("pt-PT"), "pt-pt");
        assert_eq!(bing_code("tl"), "fil");
        assert_eq!(bing_code("auto"), "auto-detect");
    }

    #[test]
    fn aliases_normalise() {
        assert_eq!(normalise("zh"), "zh-CN");
        assert_eq!(normalise("zh-Hant"), "zh-TW");
        assert_eq!(google_code("zh-TW"), "zh-TW");
    }

    #[test]
    fn right_to_left_languages_are_flagged() {
        assert!(is_rtl("ar"));
        assert!(is_rtl("he"));
        assert!(!is_rtl("vi"));
    }
}
