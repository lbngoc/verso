//! T046 — `http-bing` contract: bootstrap parsing, token refresh, error mapping, single request.

use std::time::{Duration, Instant};

use verso_lib::core::engine::{TranslateEngine, TranslationRequest};
use verso_lib::core::errors::EngineError;
use verso_lib::engines::http_bing::HttpBingEngine;
use verso_lib::engines::BROWSER_USER_AGENT;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const BOOTSTRAP: &str = r#"
    <html><head><script>IG:"ABC123",</script></head>
    <body><div id="rich_tta" data-iid="translator.5023"></div>
    <script>params_AbusePreventionHelper = [1700000000000,"token%2Bvalue",3600000];</script>
    </body></html>
"#;

const ANSWER: &str =
    r#"[{"detectedLanguage":{"language":"en"},"translations":[{"text":"Xin chào"}]}]"#;

async fn server_with_bootstrap() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translator"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BOOTSTRAP))
        .mount(&server)
        .await;
    server
}

fn request(text: &str) -> TranslationRequest {
    TranslationRequest::new(text, "auto", "vi")
}

#[tokio::test]
async fn bootstraps_and_translates_in_one_request() {
    let server = server_with_bootstrap().await;
    Mock::given(method("POST"))
        .and(path("/ttranslatev3"))
        .and(body_string_contains("text=hello"))
        .and(body_string_contains("fromLang=auto-detect"))
        .and(body_string_contains("to=vi"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ANSWER))
        .mount(&server)
        .await;

    let engine = HttpBingEngine::with_base_url_unchecked(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(8))
        .await
        .expect("translation");
    assert_eq!(result.text, "Xin chào");
    assert_eq!(result.source_lang, "en");

    let requests = server.received_requests().await.unwrap();
    let posts = requests
        .iter()
        .filter(|request| request.url.path() == "/ttranslatev3")
        .count();
    assert_eq!(
        posts, 1,
        "one request per translation at the 1,000-char cap"
    );
}

#[tokio::test]
async fn a_status_code_body_is_never_shown_as_a_translation() {
    let server = server_with_bootstrap().await;
    Mock::given(method("POST"))
        .and(path("/ttranslatev3"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"statusCode":400}"#))
        .mount(&server)
        .await;

    let engine = HttpBingEngine::with_base_url_unchecked(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(8))
        .await;
    assert_eq!(result, Err(EngineError::ParseError));
}

#[tokio::test]
async fn a_429_is_rate_limited() {
    let server = server_with_bootstrap().await;
    Mock::given(method("POST"))
        .and(path("/ttranslatev3"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let engine = HttpBingEngine::with_base_url_unchecked(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(8))
        .await;
    assert_eq!(result, Err(EngineError::RateLimited));
}

#[tokio::test]
async fn a_client_error_refreshes_the_session_and_retries_once() {
    let server = server_with_bootstrap().await;
    Mock::given(method("POST"))
        .and(path("/ttranslatev3"))
        .respond_with(ResponseTemplate::new(403))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/ttranslatev3"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ANSWER))
        .mount(&server)
        .await;

    let engine = HttpBingEngine::with_base_url_unchecked(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(8))
        .await
        .expect("translation after a session refresh");
    assert_eq!(result.text, "Xin chào");

    let requests = server.received_requests().await.unwrap();
    let bootstraps = requests
        .iter()
        .filter(|request| request.url.path() == "/translator")
        .count();
    assert!(
        bootstraps >= 2,
        "the session is bootstrapped again after a 4xx"
    );
}

#[tokio::test]
async fn one_thousand_cjk_characters_are_one_request_within_the_timeout() {
    let server = server_with_bootstrap().await;
    Mock::given(method("POST"))
        .and(path("/ttranslatev3"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(6_600))
                .set_body_string(ANSWER),
        )
        .mount(&server)
        .await;

    let engine = HttpBingEngine::with_base_url_unchecked(server.uri()).unwrap();
    let text = "汉".repeat(1_000);
    let started = Instant::now();
    let result = engine
        .translate(&request(&text), Duration::from_secs(8))
        .await
        .expect("the 8 s timeout covers the measured 6.6 s");
    assert_eq!(result.text, "Xin chào");
    assert!(started.elapsed() < Duration::from_secs(8));

    let requests = server.received_requests().await.unwrap();
    let posts = requests
        .iter()
        .filter(|request| request.url.path() == "/ttranslatev3")
        .count();
    assert_eq!(posts, 1);
}

#[tokio::test]
async fn plain_http_is_rejected() {
    assert!(HttpBingEngine::with_base_url("http://www.bing.com").is_err());
}

async fn count(server: &MockServer, request_path: &str) -> usize {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path() == request_path)
        .count()
}

#[tokio::test]
async fn a_429_is_not_retried_inside_the_engine() {
    // The registry retries a 429 once after a backoff; a second immediate POST would only add load.
    let server = server_with_bootstrap().await;
    Mock::given(method("POST"))
        .and(path("/ttranslatev3"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let engine = HttpBingEngine::with_base_url_unchecked(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(8))
        .await;
    assert_eq!(result, Err(EngineError::RateLimited));
    assert_eq!(count(&server, "/ttranslatev3").await, 1);
}

#[tokio::test]
async fn a_refusal_that_survives_a_fresh_session_is_blocked() {
    // What Bing answered a bare `Mozilla/5.0` client on 2026-11-09.
    let server = server_with_bootstrap().await;
    Mock::given(method("POST"))
        .and(path("/ttranslatev3"))
        .respond_with(ResponseTemplate::new(401).set_body_string(r#"{"ShowCaptcha":false}"#))
        .mount(&server)
        .await;

    let engine = HttpBingEngine::with_base_url_unchecked(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(8))
        .await;
    assert_eq!(result, Err(EngineError::Blocked));
    assert_eq!(
        count(&server, "/ttranslatev3").await,
        2,
        "one retry with a fresh session"
    );
    assert_eq!(count(&server, "/translator").await, 2);
}

#[tokio::test]
async fn a_captcha_answer_is_blocked() {
    let server = server_with_bootstrap().await;
    Mock::given(method("POST"))
        .and(path("/ttranslatev3"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ShowCaptcha":true}"#))
        .mount(&server)
        .await;

    let engine = HttpBingEngine::with_base_url_unchecked(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(8))
        .await;
    assert_eq!(result, Err(EngineError::Blocked));
}

#[tokio::test]
async fn sends_the_browser_user_agent_on_both_requests() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translator"))
        .and(browser_user_agent)
        .respond_with(ResponseTemplate::new(200).set_body_string(BOOTSTRAP))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/ttranslatev3"))
        .and(browser_user_agent)
        .respond_with(ResponseTemplate::new(200).set_body_string(ANSWER))
        .mount(&server)
        .await;

    let engine = HttpBingEngine::with_base_url_unchecked(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(8))
        .await
        .expect("the mock only answers the browser User-Agent");
    assert_eq!(result.text, "Xin chào");
}

/// wiremock's `header` matcher splits values on commas, and the User-Agent has one.
fn browser_user_agent(request: &wiremock::Request) -> bool {
    request
        .headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        == Some(BROWSER_USER_AGENT)
}
