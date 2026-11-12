//! Translation pipeline: 1,000-char cap → engines in order → same-language redirect → result.
//!
//! Implements FR-013 (cap + notice data), FR-014 (secondary language) and the engine-contract rules.

use std::sync::Arc;

use log::debug;

use super::engine::{Direction, Origin, TranslationRequest, TranslationResult};
use super::errors::EngineError;
use super::langdetect;
use super::languages;
use super::registry::Registry;
use super::settings::SettingsStore;

/// FR-013: the MVP translates at most 1,000 Unicode code points per request.
pub const MAX_INPUT_CHARS: usize = 1_000;

pub struct Pipeline {
    registry: Arc<Registry>,
    settings: Arc<SettingsStore>,
}

impl Pipeline {
    pub fn new(registry: Arc<Registry>, settings: Arc<SettingsStore>) -> Self {
        Self { registry, settings }
    }

    pub fn registry(&self) -> &Arc<Registry> {
        &self.registry
    }

    pub fn settings(&self) -> &Arc<SettingsStore> {
        &self.settings
    }

    /// The UI passes raw text; the pipeline owns trimming, capping and language direction.
    pub async fn translate(
        &self,
        text: &str,
        source: &str,
        target: &str,
        origin: Origin,
    ) -> Result<TranslationResult, EngineError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(EngineError::EmptyInput);
        }

        let settings = self.settings.get();
        let total_chars = trimmed.chars().count();
        let (capped, truncated) = if total_chars > MAX_INPUT_CHARS {
            (
                trimmed.chars().take(MAX_INPUT_CHARS).collect::<String>(),
                true,
            )
        } else {
            (trimmed.to_string(), false)
        };

        let requested_source = languages::normalise(source);
        let mut requested_target = languages::normalise(target);
        if requested_target.is_empty() || requested_target == "auto" {
            requested_target = settings.target_lang.clone();
        }

        // FR-014: detected (or explicit) source equal to the target → secondary language.
        let detected = if requested_source == "auto" {
            langdetect::detect_language(&capped)
        } else {
            Some(requested_source.clone())
        };
        if detected.as_deref() == Some(requested_target.as_str()) {
            requested_target = if requested_target == settings.target_lang {
                settings.secondary_lang.clone()
            } else {
                // The caller asked for a target that matches the source; redirect to the default
                // secondary language for that target (English, or Vietnamese when target is English).
                if requested_target == "en" {
                    "vi".into()
                } else {
                    "en".into()
                }
            };
            debug!(
                "source equals target; redirecting to secondary language {}",
                requested_target
            );
        }

        let request = TranslationRequest {
            text: capped,
            source: requested_source.clone(),
            target: requested_target.clone(),
            origin,
            truncated,
            original_length: if truncated { Some(total_chars) } else { None },
        };

        let mut result = self.registry.translate(&request).await?;

        // Engine-reported language first; local detection as the fallback (FR-005, research P10).
        if result.source_lang == "auto" || result.source_lang.is_empty() {
            if let Some(language) = langdetect::detect_language(&request.text) {
                result.source_lang = language;
            } else if requested_source != "auto" {
                result.source_lang = requested_source;
            }
        }
        result.target_lang = requested_target.clone();
        result.direction_used = Direction {
            source: result.source_lang.clone(),
            target: result.target_lang.clone(),
        };
        result.truncated = truncated;
        result.original_length = if truncated { Some(total_chars) } else { None };
        result.from_cache = false;
        result.offline = false;
        result.dictionary = None;
        Ok(result)
    }

    /// Convenience for `translate_selection`: the user's current target language.
    pub async fn translate_selection_text(
        &self,
        text: &str,
        source: &str,
    ) -> Result<TranslationResult, EngineError> {
        let target = self.settings.get().target_lang;
        self.translate(text, source, &target, Origin::Selection)
            .await
    }

    pub async fn translate_typed(&self, text: &str) -> Result<TranslationResult, EngineError> {
        let settings = self.settings.get();
        self.translate(
            text,
            &settings.source_lang,
            &settings.target_lang,
            Origin::Typed,
        )
        .await
    }
}
