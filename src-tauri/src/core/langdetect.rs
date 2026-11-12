//! Local language detection (research P10): engine-reported language first, `whatlang` otherwise.

use serde::{Deserialize, Serialize};
use whatlang::{detect, Lang};

/// Text shorter than this is not detected reliably (single letters, "ok", "hi").
pub const MIN_DETECTABLE_CHARS: usize = 2;
/// Below this confidence the detector's answer is not shown as a language.
pub const DEFAULT_MIN_CONFIDENCE: f64 = 0.1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    /// BCP-47-ish code, e.g. `en`, `vi`, `zh`.
    pub language: String,
    pub confidence: f64,
    pub reliable: bool,
}

/// Detects the language of `text`, returning `None` when the text is too short or ambiguous.
pub fn detect_language(text: &str) -> Option<String> {
    detect_with_confidence(text, DEFAULT_MIN_CONFIDENCE).map(|detection| detection.language)
}

pub fn detect_with_confidence(text: &str, min_confidence: f64) -> Option<Detection> {
    let trimmed = text.trim();
    if trimmed.chars().count() < MIN_DETECTABLE_CHARS {
        return None;
    }
    let info = detect(trimmed)?;
    if info.confidence() < min_confidence {
        return None;
    }
    Some(Detection {
        language: bcp47(info.lang()),
        confidence: info.confidence(),
        reliable: info.is_reliable(),
    })
}

/// Maps `whatlang` languages to the codes the engines and the UI speak.
///
/// Only the languages Verso names in its picker get a short code here; everything else falls
/// back to whatlang's ISO 639-3 code, which Google accepts.
pub fn bcp47(lang: Lang) -> String {
    match lang {
        Lang::Eng => "en",
        Lang::Vie => "vi",
        Lang::Cmn => "zh",
        Lang::Jpn => "ja",
        Lang::Kor => "ko",
        Lang::Rus => "ru",
        Lang::Ara => "ar",
        Lang::Heb => "he",
        Lang::Fra => "fr",
        Lang::Deu => "de",
        Lang::Spa => "es",
        Lang::Por => "pt",
        Lang::Ita => "it",
        Lang::Nld => "nl",
        Lang::Pol => "pl",
        Lang::Ukr => "uk",
        Lang::Ces => "cs",
        Lang::Swe => "sv",
        Lang::Dan => "da",
        Lang::Fin => "fi",
        Lang::Ell => "el",
        Lang::Tur => "tr",
        Lang::Tha => "th",
        Lang::Ind => "id",
        Lang::Hin => "hi",
        Lang::Ben => "bn",
        Lang::Tam => "ta",
        Lang::Tel => "te",
        Lang::Mal => "ml",
        Lang::Mar => "mr",
        Lang::Guj => "gu",
        Lang::Pan => "pa",
        Lang::Urd => "ur",
        Lang::Tgl => "tl",
        Lang::Kat => "ka",
        Lang::Uzb => "uz",
        Lang::Sin => "si",
        Lang::Nep => "ne",
        Lang::Khm => "km",
        Lang::Mya => "my",
        Lang::Afr => "af",
        Lang::Hrv => "hr",
        Lang::Srp => "sr",
        Lang::Slk => "sk",
        Lang::Slv => "sl",
        Lang::Bul => "bg",
        Lang::Lit => "lt",
        Lang::Lav => "lv",
        Lang::Est => "et",
        Lang::Ron => "ro",
        Lang::Hun => "hu",
        Lang::Aze => "az",
        Lang::Hye => "hy",
        other => other.code(),
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_the_mvp_languages() {
        let cases = [
            ("This is a simple English sentence about translation.", "en"),
            ("Đây là một câu tiếng Việt đơn giản về dịch thuật.", "vi"),
            ("这是一个关于翻译的简单中文句子。", "zh"),
            ("これは翻訳についての簡単な日本語の文です。", "ja"),
            ("이것은 번역에 관한 간단한 한국어 문장입니다.", "ko"),
            ("Это простое предложение на русском языке о переводе.", "ru"),
            ("هذه جملة عربية بسيطة عن الترجمة.", "ar"),
        ];
        for (text, expected) in cases {
            assert_eq!(detect_language(text).as_deref(), Some(expected), "{text}");
        }
    }

    #[test]
    fn short_text_is_below_the_threshold() {
        assert!(detect_language("a").is_none());
        assert!(detect_language("").is_none());
        assert!(detect_language("   ").is_none());
    }

    #[test]
    fn a_high_confidence_threshold_can_hide_a_result() {
        // The detector knows this is English, but a 0.999 threshold is not met by a short phrase.
        let detection = detect_with_confidence("Hello", 0.999);
        assert!(detection.is_none() || detection.unwrap().confidence >= 0.999);
    }
}
