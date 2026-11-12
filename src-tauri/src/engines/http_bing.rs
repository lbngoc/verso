//! `http-bing` — keyless Microsoft/Bing adapter (port of BingTranslator.awk).
//!
//! Session bootstrap (`/translator` → `IG`, `data-iid`, `params_AbusePreventionHelper`) is lazy and
//! kept in memory only; a translation is a single `POST /ttranslatev3` for at most 1,000 chars
//! (research P2). HTTPS is mandatory (FR-015).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use log::debug;
use parking_lot::Mutex;
use reqwest::{Client, Url};
use serde_json::Value;

use crate::core::engine::{Availability, TranslateEngine, TranslationRequest, TranslationResult};
use crate::core::errors::EngineError;
use crate::core::languages;

pub const ENGINE_ID: &str = "http-bing";
pub const DEFAULT_BASE_URL: &str = "https://www.bing.com";
pub const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BingSession {
    pub ig: String,
    pub iid: String,
    pub key: String,
    pub token: String,
    pub expires_at: SystemTime,
}

impl BingSession {
    pub fn is_expired(&self, now: SystemTime) -> bool {
        now >= self.expires_at
    }
}

/// Parses the translator page into the values the POST needs. Pure, so fixtures can drive it.
pub fn parse_bootstrap(html: &str) -> Result<BingSession, EngineError> {
    let ig = between(html, "IG:\"", "\"").ok_or(EngineError::ParseError)?;
    let iid = between(html, "data-iid=\"", "\"").ok_or(EngineError::ParseError)?;
    let params = between(html, "params_AbusePreventionHelper = [", "]")
        .or_else(|| between(html, "params_AbusePreventionHelper = [", "];"))
        .ok_or(EngineError::ParseError)?;

    let params = format!("[{params}]");
    let parsed: Value = serde_json::from_str(&params).map_err(|_| EngineError::ParseError)?;
    let array = parsed.as_array().ok_or(EngineError::ParseError)?;
    let key = match array.first() {
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::String(value)) => value.clone(),
        _ => return Err(EngineError::ParseError),
    };
    let token = array
        .get(1)
        .and_then(Value::as_str)
        .map(unescape_html)
        .ok_or(EngineError::ParseError)?;
    let ttl_ms = array
        .get(2)
        .and_then(Value::as_u64)
        .unwrap_or(60 * 60 * 1000)
        .max(1_000);

    Ok(BingSession {
        ig: unescape_html(ig),
        iid: unescape_html(iid),
        key: unescape_html(&key),
        token,
        expires_at: SystemTime::now() + Duration::from_millis(ttl_ms),
    })
}

/// Reads `[0].translations[0].text` and `[0].detectedLanguage.language`.
pub fn parse_translation(body: &str, target: &str) -> Result<TranslationResult, EngineError> {
    let trimmed = body.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('<') {
        return Err(EngineError::ParseError);
    }
    let value: Value = serde_json::from_str(body).map_err(|_| EngineError::ParseError)?;

    if value.get("ShowCaptcha").and_then(Value::as_bool) == Some(true) {
        return Err(EngineError::Blocked);
    }
    if let Some(status) = value.get("statusCode").and_then(Value::as_u64) {
        return Err(match status {
            429 => EngineError::RateLimited,
            401 | 403 => EngineError::Blocked,
            _ => EngineError::ParseError,
        });
    }

    let root = value.as_array().ok_or(EngineError::ParseError)?;
    let first = root.first().ok_or(EngineError::ParseError)?;
    let translation = first
        .get("translations")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or(EngineError::ParseError)?;
    let text = translation
        .get("text")
        .and_then(Value::as_str)
        .ok_or(EngineError::ParseError)?;
    let detected = first
        .get("detectedLanguage")
        .and_then(|value| value.get("language"))
        .and_then(Value::as_str)
        .map(str::to_string);

    Ok(TranslationResult::from_engine(
        ENGINE_ID,
        text,
        detected,
        languages::normalise(target),
    ))
}

fn between<'a>(haystack: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = haystack.find(start)? + start.len();
    let rest = &haystack[from..];
    let to = rest.find(end)?;
    Some(&rest[..to])
}

fn unescape_html(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

pub struct HttpBingEngine {
    client: Client,
    base_url: String,
    session: Mutex<Option<BingSession>>,
    sequence: AtomicU64,
}

impl HttpBingEngine {
    pub fn new() -> Result<Self, EngineError> {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    /// Production constructor: HTTPS only (FR-015).
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self, EngineError> {
        let base_url = base_url.into();
        let parsed = Url::parse(&base_url)
            .map_err(|_| EngineError::internal("http-bing base URL is not a URL"))?;
        if parsed.scheme() != "https" {
            return Err(EngineError::internal(
                "http-bing requires an encrypted (https) connection",
            ));
        }
        Self::build(base_url)
    }

    /// Test seam for local mock servers (wiremock speaks plain http). Never used in production.
    #[doc(hidden)]
    pub fn with_base_url_unchecked(base_url: impl Into<String>) -> Result<Self, EngineError> {
        Self::build(base_url.into())
    }

    fn build(base_url: String) -> Result<Self, EngineError> {
        let client = Client::builder()
            .user_agent(super::BROWSER_USER_AGENT)
            .timeout(TIMEOUT)
            .pool_max_idle_per_host(1)
            .cookie_store(true)
            .build()
            .map_err(EngineError::from)?;
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            session: Mutex::new(None),
            sequence: AtomicU64::new(0),
        })
    }

    fn cached_session(&self) -> Option<BingSession> {
        let guard = self.session.lock();
        match guard.as_ref() {
            Some(session) if !session.is_expired(SystemTime::now()) => Some(session.clone()),
            _ => None,
        }
    }

    fn invalidate(&self) {
        *self.session.lock() = None;
    }

    async fn bootstrap(&self, timeout: Duration) -> Result<BingSession, EngineError> {
        let url = format!("{}/translator", self.base_url);
        let response = self
            .client
            .get(url)
            .timeout(timeout.min(TIMEOUT))
            .send()
            .await
            .map_err(EngineError::from)?;
        if !response.status().is_success() {
            return Err(EngineError::ParseError);
        }
        let html = response.text().await.map_err(EngineError::from)?;
        let session = parse_bootstrap(&html)?;
        *self.session.lock() = Some(session.clone());
        Ok(session)
    }

    async fn session(&self, timeout: Duration) -> Result<BingSession, EngineError> {
        if let Some(session) = self.cached_session() {
            return Ok(session);
        }
        self.bootstrap(timeout).await
    }

    async fn request(
        &self,
        request: &TranslationRequest,
        session: &BingSession,
        timeout: Duration,
    ) -> Result<TranslationResult, EngineError> {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let url = format!(
            "{}/ttranslatev3?isVertical=1&IG={}&IID={}.{}",
            self.base_url, session.ig, session.iid, sequence
        );
        let source = if request.source == "auto" {
            "auto-detect".to_string()
        } else {
            languages::bing_code(&request.source)
        };
        let target = languages::bing_code(&request.target);
        let form = [
            ("fromLang", source),
            ("to", target),
            ("text", request.text.clone()),
            ("token", session.token.clone()),
            ("key", session.key.clone()),
        ];

        debug!(
            "http-bing POST /ttranslatev3 ({} chars)",
            request.text.chars().count()
        );
        let response = self
            .client
            .post(url)
            .form(&form)
            .timeout(timeout.min(TIMEOUT))
            .send()
            .await
            .map_err(EngineError::from)?;

        let status = response.status();
        if status.as_u16() == 429 {
            return Err(EngineError::RateLimited);
        }
        let body = response.text().await.map_err(EngineError::from)?;
        if status.is_client_error() {
            // 401/403: the session was refused, or the client was (`{"ShowCaptcha":…}`). The caller
            // retries once with a fresh session; if that is refused too, this network is blocked.
            // Other 4xx are answers this adapter does not understand.
            return Err(match status.as_u16() {
                401 | 403 => EngineError::Blocked,
                _ => parse_translation(&body, &request.target)
                    .err()
                    .unwrap_or(EngineError::ParseError),
            });
        }
        if !status.is_success() {
            return Err(EngineError::ParseError);
        }
        parse_translation(&body, &request.target)
    }
}

#[async_trait]
impl TranslateEngine for HttpBingEngine {
    fn id(&self) -> &'static str {
        ENGINE_ID
    }

    fn service(&self) -> &'static str {
        "Microsoft"
    }

    fn availability(&self) -> Availability {
        Availability::Available
    }

    fn languages(&self) -> Vec<String> {
        languages::all()
            .iter()
            .filter(|language| language.bing.is_some())
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

        let session = self.session(timeout.min(TIMEOUT)).await?;
        match self.request(request, &session, timeout).await {
            Ok(result) => Ok(result),
            // A refused or unreadable answer gets one fresh session, then the registry moves on.
            // `RateLimited` is not retried here: the registry already retries a 429 once after a
            // backoff, and an immediate second request only deepens the throttle.
            Err(error @ (EngineError::Blocked | EngineError::ParseError)) => {
                self.invalidate();
                let refreshed = self.bootstrap(timeout.min(TIMEOUT)).await?;
                match self.request(request, &refreshed, timeout).await {
                    Ok(result) => Ok(result),
                    Err(_) => Err(error),
                }
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bootstrap_page() {
        let html = r#"
            <script>IG:"ABC123",</script>
            <div id="rich_tta" data-iid="translator.5023"></div>
            params_AbusePreventionHelper = [1700000000000,"tok%2Ben",3600000];
        "#;
        let session = parse_bootstrap(html).unwrap();
        assert_eq!(session.ig, "ABC123");
        assert_eq!(session.iid, "translator.5023");
        assert_eq!(session.key, "1700000000000");
        assert_eq!(session.token, "tok%2Ben");
    }

    #[test]
    fn rejects_pages_without_session_data() {
        assert_eq!(
            parse_bootstrap("<html></html>"),
            Err(EngineError::ParseError)
        );
    }

    #[test]
    fn parses_translation_answer() {
        let body =
            r#"[{"detectedLanguage":{"language":"en"},"translations":[{"text":"Xin chào"}]}]"#;
        let result = parse_translation(body, "vi").unwrap();
        assert_eq!(result.text, "Xin chào");
        assert_eq!(result.source_lang, "en");
    }

    #[test]
    fn maps_status_code_bodies() {
        assert_eq!(
            parse_translation(r#"{"statusCode":400}"#, "vi"),
            Err(EngineError::ParseError)
        );
        assert_eq!(
            parse_translation(r#"{"statusCode":429}"#, "vi"),
            Err(EngineError::RateLimited)
        );
    }

    #[test]
    fn rejects_plain_http_base_url() {
        assert!(HttpBingEngine::with_base_url("http://www.bing.com").is_err());
        assert!(HttpBingEngine::with_base_url("https://www.bing.com").is_ok());
    }
}
