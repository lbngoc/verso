//! T047 — fallback rules: retry once on 429, cooldown, events, aggregated errors, fake 429.

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use verso_lib::core::engine::TranslationRequest;
use verso_lib::core::errors::EngineError;
use verso_lib::core::registry::{Registry, COOLDOWN};

fn request() -> TranslationRequest {
    TranslationRequest::new("hello", "en", "vi")
}

#[tokio::test]
async fn a_429_is_retried_once_then_the_next_engine_answers() {
    let (dir, settings) = common::temp_store();
    let google = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::error(EngineError::RateLimited),
    ));
    let bing = Arc::new(common::FakeEngine::new(
        "http-bing",
        common::FakeResponse::ok("xin chào", Some("en")),
    ));
    let registry = Registry::new(vec![google.clone(), bing.clone()], settings);

    let started = Instant::now();
    let result = registry.translate(&request()).await.expect("translation");
    assert_eq!(result.engine_id, "http-bing");
    assert_eq!(google.calls(), 2, "429 must be retried exactly once");
    assert_eq!(bing.calls(), 1);
    assert!(
        started.elapsed() >= Duration::from_millis(380),
        "the retry waits 400–800 ms"
    );
    drop(dir);
}

#[tokio::test]
async fn a_failed_engine_is_cooled_down_for_sixty_seconds() {
    let (_dir, settings) = common::temp_store();
    let google = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::error(EngineError::RateLimited),
    ));
    let bing = Arc::new(common::FakeEngine::new(
        "http-bing",
        common::FakeResponse::ok("xin chào", Some("en")),
    ));
    let registry =
        Registry::new(vec![google.clone(), bing.clone()], settings).with_backoff_ms(1..=2);

    registry.translate(&request()).await.expect("first");
    let google_after_first = google.calls();
    assert_eq!(google_after_first, 2);

    registry.translate(&request()).await.expect("second");
    assert_eq!(
        google.calls(),
        google_after_first,
        "Google stays in cooldown for {} s",
        COOLDOWN.as_secs()
    );
    assert_eq!(bing.calls(), 2);
}

#[tokio::test]
async fn a_fallback_event_names_both_engines_and_the_reason() {
    let (_dir, settings) = common::temp_store();
    let google = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::error(EngineError::NoConnection),
    ));
    let bing = Arc::new(common::FakeEngine::new(
        "http-bing",
        common::FakeResponse::ok("xin chào", Some("en")),
    ));
    let registry = Registry::new(vec![google, bing], settings);
    let events: Arc<Mutex<Vec<(String, String, EngineError)>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let events = events.clone();
        registry.set_fallback_sink(Arc::new(move |from, to, error| {
            events
                .lock()
                .push((from.to_string(), to.to_string(), error.clone()));
        }));
    }

    registry.translate(&request()).await.expect("translation");
    let events = events.lock().clone();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "http-google");
    assert_eq!(events[0].1, "http-bing");
    assert_eq!(events[0].2, EngineError::NoConnection);
}

#[tokio::test]
async fn all_failures_aggregate_to_the_most_actionable_error() {
    let (_dir, settings) = common::temp_store();
    let google = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::error(EngineError::NoConnection),
    ));
    let bing = Arc::new(common::FakeEngine::new(
        "http-bing",
        common::FakeResponse::error(EngineError::ParseError),
    ));
    let registry = Registry::new(vec![google, bing], settings);

    assert_eq!(
        registry.translate(&request()).await,
        Err(EngineError::NoConnection)
    );
}

#[tokio::test]
async fn the_debug_fake_429_hook_forces_a_fallback() {
    let (_dir, settings) = common::temp_store();
    let google = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::ok("google answer", Some("en")),
    ));
    let bing = Arc::new(common::FakeEngine::new(
        "http-bing",
        common::FakeResponse::ok("bing answer", Some("en")),
    ));
    let registry = Registry::new(vec![google.clone(), bing.clone()], settings)
        .with_fake_429(["http-google"])
        .with_backoff_ms(1..=2);

    let result = registry.translate(&request()).await.expect("translation");
    assert_eq!(result.engine_id, "http-bing");
    // The hook answers before the engine is called, so the fake engine stays untouched; the
    // attempts list still records both forced 429s.
    assert_eq!(
        result
            .attempts
            .iter()
            .filter(|attempt| attempt.engine_id == "http-google")
            .count(),
        2
    );
    assert_eq!(google.calls(), 0);
    assert_eq!(bing.calls(), 1);
}

#[tokio::test]
async fn engine_metadata_reflects_order_and_cooldown() {
    let (_dir, settings) = common::temp_store();
    let google = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::error(EngineError::RateLimited),
    ));
    let bing = Arc::new(common::FakeEngine::new(
        "http-bing",
        common::FakeResponse::ok("xin chào", Some("en")),
    ));
    let registry = Registry::new(vec![google, bing], settings).with_backoff_ms(1..=2);
    registry.translate(&request()).await.expect("translation");

    let infos = registry.engine_infos();
    assert_eq!(infos[0].id, "http-google");
    assert!(infos[0].cooldown_remaining_s > 0);
    assert_eq!(infos[1].id, "http-bing");
}
