//! Parser for Google's gtx answer (`translate_a/single`).
//!
//! Ported from translate-shell `GoogleTranslate.awk` (Unlicense): the translated text is the
//! concatenation of `[0][i][0]` and the detected language is `[2]`.

use serde_json::Value;

use crate::core::errors::EngineError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GtxAnswer {
    pub text: String,
    pub detected_lang: Option<String>,
}

pub fn parse_gtx(body: &str) -> Result<GtxAnswer, EngineError> {
    let trimmed = body.trim_start();
    // Google answers throttling with an HTML page; never surface it (contract § http-google).
    if trimmed.is_empty() || trimmed.starts_with('<') {
        return Err(EngineError::ParseError);
    }
    let value: Value = serde_json::from_str(body).map_err(|_| EngineError::ParseError)?;
    let root = value.as_array().ok_or(EngineError::ParseError)?;
    let segments = root
        .first()
        .and_then(Value::as_array)
        .ok_or(EngineError::ParseError)?;

    let mut text = String::new();
    for segment in segments {
        if let Some(piece) = segment
            .as_array()
            .and_then(|parts| parts.first())
            .and_then(Value::as_str)
        {
            text.push_str(piece);
        }
    }
    if text.is_empty() {
        return Err(EngineError::ParseError);
    }

    let detected_lang = root.get(2).and_then(Value::as_str).map(str::to_string);
    Ok(GtxAnswer {
        text,
        detected_lang,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_segments_and_reads_detected_language() {
        let body = r#"[[["Hello ","Xin chào "],["world","thế giới"]],null,"en",null,10]"#;
        let answer = parse_gtx(body).unwrap();
        assert_eq!(answer.text, "Hello world");
        assert_eq!(answer.detected_lang.as_deref(), Some("en"));
    }

    #[test]
    fn rejects_html_and_empty_bodies() {
        assert_eq!(
            parse_gtx("<html>unusual traffic</html>"),
            Err(EngineError::ParseError)
        );
        assert_eq!(parse_gtx(""), Err(EngineError::ParseError));
        assert_eq!(parse_gtx("  \n"), Err(EngineError::ParseError));
    }

    #[test]
    fn rejects_json_that_is_not_an_answer() {
        assert_eq!(parse_gtx("{}"), Err(EngineError::ParseError));
        assert_eq!(parse_gtx("[]"), Err(EngineError::ParseError));
        assert_eq!(parse_gtx(r#"[[]]"#), Err(EngineError::ParseError));
    }

    #[test]
    fn handles_missing_detected_language() {
        let answer = parse_gtx(r#"[[["hi",null]],null]"#).unwrap();
        assert_eq!(answer.text, "hi");
        assert_eq!(answer.detected_lang, None);
    }
}
