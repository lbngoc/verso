//! T021 — engine contract tests for `http-google` against a local mock server.

use std::time::Duration;

use verso_lib::core::engine::{TranslateEngine, TranslationRequest};
use verso_lib::core::errors::EngineError;
use verso_lib::engines::http_google::HttpGoogleEngine;
use verso_lib::engines::BROWSER_USER_AGENT;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn request(text: &str) -> TranslationRequest {
    TranslationRequest::new(text, "auto", "vi")
}

#[tokio::test]
async fn empty_text_is_rejected_without_a_request() {
    let engine = HttpGoogleEngine::with_base_url("http://127.0.0.1:1").unwrap();
    let result = engine
        .translate(&request("   \n "), Duration::from_secs(1))
        .await;
    assert_eq!(result, Err(EngineError::EmptyInput));
}

#[tokio::test]
async fn timeout_is_honoured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(600))
                .set_body_string(r#"[[["xin chào",null]],null,"en"]"#),
        )
        .mount(&server)
        .await;

    let engine = HttpGoogleEngine::with_base_url(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_millis(80))
        .await;
    assert_eq!(result, Err(EngineError::Timeout));
}

#[tokio::test]
async fn a_429_is_rate_limited() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let engine = HttpGoogleEngine::with_base_url(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(2))
        .await;
    assert_eq!(result, Err(EngineError::RateLimited));
}

#[tokio::test]
async fn an_html_page_is_never_surfaced() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "<html><body>Our systems have detected unusual traffic</body></html>",
            ),
        )
        .mount(&server)
        .await;

    let engine = HttpGoogleEngine::with_base_url(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(2))
        .await;
    assert_eq!(result, Err(EngineError::ParseError));
}

#[tokio::test]
async fn text_travels_in_the_query_and_unicode_round_trips() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .and(query_param("q", "مرحبا بالعالم"))
        .and(query_param("client", "gtx"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"[[["مرحبا بالعالم",null]],null,"ar"]"#),
        )
        .mount(&server)
        .await;

    let engine = HttpGoogleEngine::with_base_url(server.uri()).unwrap();
    let mut request = request("مرحبا بالعالم");
    request.target = "ar".into();
    let result = engine
        .translate(&request, Duration::from_secs(2))
        .await
        .expect("translation");
    assert_eq!(result.text, "مرحبا بالعالم");
    assert_eq!(result.source_lang, "ar");
}

#[tokio::test]
async fn detected_language_is_parsed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"[[["Xin chào ",null],["thế giới",null]],null,"en"]"#),
        )
        .mount(&server)
        .await;

    let engine = HttpGoogleEngine::with_base_url(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello world"), Duration::from_secs(2))
        .await
        .expect("translation");
    assert_eq!(result.text, "Xin chào thế giới");
    assert_eq!(result.source_lang, "en");
}

#[tokio::test]
async fn a_connection_failure_is_reported_as_no_connection() {
    let engine = HttpGoogleEngine::with_base_url("http://127.0.0.1:9").unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_millis(300))
        .await;
    assert!(matches!(
        result,
        Err(EngineError::NoConnection) | Err(EngineError::Timeout)
    ));
}

#[tokio::test]
async fn the_unusual_traffic_page_is_blocked_not_busy() {
    // Google redirects a flagged network to its CAPTCHA page, which answers 429.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .respond_with(ResponseTemplate::new(302).insert_header(
            "Location",
            format!("{}/sorry/index?continue=x", server.uri()),
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/sorry/index"))
        .respond_with(ResponseTemplate::new(429).set_body_string("<html>unusual traffic</html>"))
        .mount(&server)
        .await;

    let engine = HttpGoogleEngine::with_base_url(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(2))
        .await;
    assert_eq!(result, Err(EngineError::Blocked));
}

#[tokio::test]
async fn sends_the_browser_user_agent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .and(browser_user_agent)
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"[[["xin chào",null]],null,"en"]"#),
        )
        .mount(&server)
        .await;

    let engine = HttpGoogleEngine::with_base_url(server.uri()).unwrap();
    let result = engine
        .translate(&request("hello"), Duration::from_secs(2))
        .await
        .expect("the mock only answers the browser User-Agent");
    assert_eq!(result.text, "xin chào");
}

/// wiremock's `header` matcher splits values on commas, and the User-Agent has one.
fn browser_user_agent(request: &wiremock::Request) -> bool {
    request
        .headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        == Some(BROWSER_USER_AGENT)
}
