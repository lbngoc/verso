use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::errors::{EngineAttempt, EngineError};

/// Where the text came from (shared data-model; 002 uses only these two).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Selection,
    Typed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub text: String,
    pub source: String,
    pub target: String,
    pub origin: Origin,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default)]
    pub original_length: Option<usize>,
}

impl TranslationRequest {
    pub fn new(
        text: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            text: text.into(),
            source: source.into(),
            target: target.into(),
            origin: Origin::Typed,
            truncated: false,
            original_length: None,
        }
    }
}

/// The direction actually used (differs from the request when FR-014 redirected).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Direction {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationResult {
    pub text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub engine_id: String,
    pub direction_used: Direction,
    #[serde(default)]
    pub attempts: Vec<EngineAttempt>,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default)]
    pub original_length: Option<usize>,
    #[serde(default)]
    pub from_cache: bool,
    #[serde(default)]
    pub offline: bool,
    #[serde(default)]
    pub dictionary: Option<serde_json::Value>,
}

impl TranslationResult {
    /// A minimal result as an adapter returns it; the pipeline fills attempts/direction.
    pub fn from_engine(
        engine_id: impl Into<String>,
        text: impl Into<String>,
        detected_lang: Option<String>,
        target_lang: impl Into<String>,
    ) -> Self {
        let target_lang = target_lang.into();
        let source_lang = detected_lang.unwrap_or_else(|| "auto".into());
        Self {
            text: text.into(),
            direction_used: Direction {
                source: source_lang.clone(),
                target: target_lang.clone(),
            },
            source_lang,
            target_lang,
            engine_id: engine_id.into(),
            attempts: Vec::new(),
            truncated: false,
            original_length: None,
            from_cache: false,
            offline: false,
            dictionary: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Availability {
    Available,
    Unavailable { reason: String },
    NeedsKey,
    NeedsPack,
}

impl Availability {
    pub fn is_available(&self) -> bool {
        matches!(self, Availability::Available)
    }
}

/// One translation service behind one common contract (engine-contract.md).
#[async_trait]
pub trait TranslateEngine: Send + Sync {
    fn id(&self) -> &'static str;
    fn service(&self) -> &'static str;
    fn availability(&self) -> Availability;
    fn languages(&self) -> Vec<String>;
    fn timeout(&self) -> Duration;
    async fn translate(
        &self,
        request: &TranslationRequest,
        timeout: Duration,
    ) -> Result<TranslationResult, EngineError>;
}

/// Metadata handed to the UI (`list_engines`, Settings › Engines).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineInfo {
    pub id: String,
    pub service: String,
    pub availability: Availability,
    pub priority: usize,
    pub cooldown_remaining_s: u64,
    pub enabled: bool,
    pub timeout_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_from_engine_sets_direction() {
        let result = TranslationResult::from_engine("http-google", "hola", Some("en".into()), "es");
        assert_eq!(result.source_lang, "en");
        assert_eq!(result.direction_used.source, "en");
        assert_eq!(result.direction_used.target, "es");
    }
}
