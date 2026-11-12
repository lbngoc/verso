//! T023 — pipeline rules: cap, empty input, same-language redirect, budget, disabled engines.

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;

use verso_lib::core::engine::{Origin, TranslateEngine};
use verso_lib::core::errors::EngineError;
use verso_lib::core::pipeline::{Pipeline, MAX_INPUT_CHARS};
use verso_lib::core::registry::Registry;

fn pipeline_with(
    engines: Vec<Arc<dyn TranslateEngine>>,
    registry_budget: Option<Duration>,
) -> (
    Pipeline,
    Arc<verso_lib::core::settings::SettingsStore>,
    tempfile::TempDir,
) {
    let (dir, settings) = common::temp_store();
    let mut registry = Registry::new(engines, settings.clone());
    if let Some(budget) = registry_budget {
        registry = registry.with_budget(budget);
    }
    let registry = Arc::new(registry);
    let pipeline = Pipeline::new(registry, settings.clone());
    (pipeline, settings, dir)
}

#[tokio::test]
async fn input_over_the_cap_is_cut_and_reported() {
    let engine = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::ok("translated", Some("en")),
    ));
    let (pipeline, _settings, _dir) = pipeline_with(vec![engine.clone()], None);
    let text = "a".repeat(MAX_INPUT_CHARS + 340);

    let result = pipeline
        .translate(&text, "en", "vi", Origin::Typed)
        .await
        .expect("translation");
    assert!(result.truncated);
    assert_eq!(result.original_length, Some(MAX_INPUT_CHARS + 340));
    assert_eq!(result.attempts.len(), 1);
    assert_eq!(engine.calls(), 1);
}

#[tokio::test]
async fn whitespace_only_input_is_rejected_before_any_engine() {
    let engine = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::ok("x", None),
    ));
    let (pipeline, _settings, _dir) = pipeline_with(vec![engine.clone()], None);

    let result = pipeline
        .translate("  \n\t ", "en", "vi", Origin::Typed)
        .await;
    assert_eq!(result, Err(EngineError::EmptyInput));
    assert_eq!(engine.calls(), 0);
}

#[tokio::test]
async fn a_source_equal_to_the_target_is_redirected_to_the_secondary_language() {
    let engine = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::ok("xin chào", Some("en")),
    ));
    let (pipeline, _settings, _dir) = pipeline_with(vec![engine], None);

    let result = pipeline
        .translate(
            "This is an English sentence that should be redirected.",
            "auto",
            "en",
            Origin::Typed,
        )
        .await
        .expect("translation");
    assert_eq!(result.direction_used.target, "vi");
    assert_eq!(result.target_lang, "vi");
}

#[tokio::test]
async fn the_total_budget_stops_the_round() {
    let slow_a = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::error(EngineError::Timeout).slow(Duration::from_millis(300)),
    ));
    let slow_b = Arc::new(common::FakeEngine::new(
        "http-bing",
        common::FakeResponse::error(EngineError::Timeout).slow(Duration::from_millis(300)),
    ));
    let (pipeline, _settings, _dir) =
        pipeline_with(vec![slow_a, slow_b], Some(Duration::from_millis(250)));

    let started = Instant::now();
    let result = pipeline.translate("hello", "en", "vi", Origin::Typed).await;
    assert_eq!(result, Err(EngineError::Timeout));
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "the 14 s budget must be injected, not waited out"
    );
}

#[tokio::test]
async fn disabled_engines_are_never_called() {
    let google = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::ok("google", Some("en")),
    ));
    let bing = Arc::new(common::FakeEngine::new(
        "http-bing",
        common::FakeResponse::ok("bing", Some("en")),
    ));
    let (pipeline, settings, _dir) = pipeline_with(vec![google.clone(), bing.clone()], None);
    settings
        .update(&json!({ "engines_disabled": ["http-google"] }))
        .unwrap();

    let result = pipeline
        .translate("hello", "en", "vi", Origin::Typed)
        .await
        .expect("translation");
    assert_eq!(result.engine_id, "http-bing");
    assert_eq!(google.calls(), 0);
    assert_eq!(bing.calls(), 1);
}

#[tokio::test]
async fn attempts_are_recorded_without_text() {
    let failing = Arc::new(common::FakeEngine::new(
        "http-google",
        common::FakeResponse::error(EngineError::RateLimited),
    ));
    let working = Arc::new(common::FakeEngine::new(
        "http-bing",
        common::FakeResponse::ok("xin chào", Some("en")),
    ));
    let (pipeline, _settings, _dir) = pipeline_with(vec![failing, working], None);

    let result = pipeline
        .translate("hello", "en", "vi", Origin::Typed)
        .await
        .expect("translation");
    assert_eq!(
        result
            .attempts
            .iter()
            .map(|attempt| attempt.engine_id.as_str())
            .collect::<Vec<_>>(),
        vec!["http-google", "http-google", "http-bing"]
    );
    let serialised = serde_json::to_string(&result.attempts).unwrap();
    assert!(!serialised.contains("hello"));
}
