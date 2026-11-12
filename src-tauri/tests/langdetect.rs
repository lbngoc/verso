//! T025 — local language detection for the quick panel and FR-014.

use verso_lib::core::langdetect::{detect_language, detect_with_confidence};

#[test]
fn detects_the_required_languages() {
    let samples = [
        ("This is a simple English sentence about translation.", "en"),
        ("Đây là một câu tiếng Việt đơn giản về dịch thuật.", "vi"),
        ("这是一个关于翻译的简单中文句子。", "zh"),
        ("これは翻訳についての簡単な日本語の文です。", "ja"),
        ("이것은 번역에 관한 간단한 한국어 문장입니다.", "ko"),
        ("Это простое предложение на русском языке о переводе.", "ru"),
        ("هذه جملة عربية بسيطة عن الترجمة.", "ar"),
    ];
    for (text, expected) in samples {
        assert_eq!(
            detect_language(text).as_deref(),
            Some(expected),
            "wrong language for: {text}"
        );
    }
}

#[test]
fn short_text_is_below_the_confidence_threshold() {
    assert!(detect_language("a").is_none());
    assert!(detect_language("").is_none());
    assert!(detect_language("   ").is_none());
}

#[test]
fn the_threshold_is_applied() {
    let detection = detect_with_confidence(
        "This is a perfectly ordinary English sentence used for detection.",
        0.99,
    );
    if let Some(detection) = detection {
        assert!(detection.confidence >= 0.99);
    }
}

#[test]
fn detection_reports_confidence_and_reliability() {
    let detection =
        detect_with_confidence("The quick brown fox jumps over the lazy dog.", 0.1).unwrap();
    assert_eq!(detection.language, "en");
    assert!(detection.confidence > 0.1);
}
