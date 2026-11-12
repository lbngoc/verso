//! `http-google` — keyless Google gtx adapter (port of GoogleTranslate.awk).
//!
//! GET only: the MVP caps input at 1,000 chars (research P3), so the query stays around 9 KB in
//! the worst case (CJK). The text is in the URL, therefore it is never logged above debug and the
//! logged excerpt is capped at 100 characters.

use std::time::Duration;

use async_trait::async_trait;
use log::debug;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::Client;

use super::gtx_parse::parse_gtx;
use crate::core::engine::{Availability, TranslateEngine, TranslationRequest, TranslationResult};
use crate::core::errors::EngineError;
use crate::core::languages;

pub const ENGINE_ID: &str = "http-google";
pub const DEFAULT_BASE_URL: &str = "https://translate.googleapis.com";
pub const TIMEOUT: Duration = Duration::from_secs(6);

pub struct HttpGoogleEngine {
    client: Client,
    base_url: String,
}

impl HttpGoogleEngine {
    pub fn new() -> Result<Self, EngineError> {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    /// Test/soak seam: point the adapter at a local mock server (must still be HTTPS in production).
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self, EngineError> {
        let client = Client::builder()
            .user_agent(super::BROWSER_USER_AGENT)
            .timeout(TIMEOUT)
            .pool_max_idle_per_host(1)
            .build()
            .map_err(EngineError::from)?;
        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }

    pub fn request_url(&self, request: &TranslationRequest) -> String {
        format!(
            "{}/translate_a/single?client=gtx&sl={}&tl={}&dt=t&ie=UTF-8&oe=UTF-8&q={}",
            self.base_url,
            encode(&languages::google_code(&request.source)),
            encode(&languages::google_code(&request.target)),
            encode(&request.text),
        )
    }
}

fn encode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

#[async_trait]
impl TranslateEngine for HttpGoogleEngine {
    fn id(&self) -> &'static str {
        ENGINE_ID
    }

    fn service(&self) -> &'static str {
        "Google"
    }

    fn availability(&self) -> Availability {
        Availability::Available
    }

    fn languages(&self) -> Vec<String> {
        languages::all()
            .iter()
            .filter(|language| language.google.is_some())
            .map(|language| language.code.clone())
            .collect()
    }

    fn timeout(&self) -> Duration {
        TIMEOUT
    }

    async fn translate(
        &self,
        request: &TranslationRequest,
        timeout: Duration,
    ) -> Result<TranslationResult, EngineError> {
        if request.text.trim().is_empty() {
            return Err(EngineError::EmptyInput);
        }
        let url = self.request_url(request);
        debug!(
            "http-google GET /translate_a/single ({} chars)",
            request.text.chars().count()
        );
        if log::log_enabled!(log::Level::Debug) {
            let excerpt: String = request.text.chars().take(100).collect();
            debug!("http-google text excerpt: {excerpt}");
        }

        let response = self
            .client
            .get(url)
            .timeout(timeout.min(TIMEOUT))
            .send()
            .await
            .map_err(EngineError::from)?;

        // Google sends throttled networks to its "unusual traffic" CAPTCHA page (`/sorry/…`); the
        // redirect is followed, so the final URL tells it apart from an ordinary 429.
        if response.url().path().starts_with("/sorry") {
            return Err(EngineError::Blocked);
        }
        let status = response.status();
        if status.as_u16() == 429 {
            return Err(EngineError::RateLimited);
        }
        if status.as_u16() == 400 || status.as_u16() == 413 {
            // >1,000 CJK chars or an unsupported parameter: the answer is not readable as JSON.
            return Err(EngineError::ParseError);
        }
        if !status.is_success() {
            return Err(EngineError::ParseError);
        }

        let body = response.text().await.map_err(EngineError::from)?;
        let answer = parse_gtx(&body)?;
        Ok(TranslationResult::from_engine(
            ENGINE_ID,
            answer.text,
            answer.detected_lang,
            languages::normalise(&request.target),
        ))
    }
}
