//! Shared test doubles: scripted engines and a temporary settings store.
#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use tempfile::TempDir;
use verso_lib::core::engine::{
    Availability, TranslateEngine, TranslationRequest, TranslationResult,
};
use verso_lib::core::errors::EngineError;
use verso_lib::core::settings::SettingsStore;

#[derive(Clone)]
pub struct FakeResponse {
    pub delay: Duration,
    pub result: Result<(String, Option<String>), EngineError>,
}

impl FakeResponse {
    pub fn ok(text: &str, detected: Option<&str>) -> Self {
        Self {
            delay: Duration::ZERO,
            result: Ok((text.to_string(), detected.map(str::to_string))),
        }
    }

    pub fn error(error: EngineError) -> Self {
        Self {
            delay: Duration::ZERO,
            result: Err(error),
        }
    }

    pub fn slow(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

#[derive(Clone)]
pub struct FakeEngine {
    id: &'static str,
    calls: Arc<AtomicUsize>,
    script: Arc<Mutex<VecDeque<FakeResponse>>>,
    fallback: FakeResponse,
}

impl FakeEngine {
    pub fn new(id: &'static str, response: FakeResponse) -> Self {
        Self {
            id,
            calls: Arc::new(AtomicUsize::new(0)),
            script: Arc::new(Mutex::new(VecDeque::new())),
            fallback: response,
        }
    }

    pub fn scripted(id: &'static str, responses: Vec<FakeResponse>) -> Self {
        let last = responses
            .last()
            .cloned()
            .unwrap_or_else(|| FakeResponse::ok("ok", None));
        Self {
            id,
            calls: Arc::new(AtomicUsize::new(0)),
            script: Arc::new(Mutex::new(responses.into())),
            fallback: last,
        }
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl TranslateEngine for FakeEngine {
    fn id(&self) -> &'static str {
        self.id
    }

    fn service(&self) -> &'static str {
        "Test"
    }

    fn availability(&self) -> Availability {
        Availability::Available
    }

    fn languages(&self) -> Vec<String> {
        vec!["en".into(), "vi".into()]
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(2)
    }

    async fn translate(
        &self,
        request: &TranslationRequest,
        _timeout: Duration,
    ) -> Result<TranslationResult, EngineError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let response = {
            let mut script = self.script.lock();
            script.pop_front().unwrap_or_else(|| self.fallback.clone())
        };
        if !response.delay.is_zero() {
            tokio::time::sleep(response.delay).await;
        }
        match response.result {
            Ok((text, detected)) => Ok(TranslationResult::from_engine(
                self.id,
                text,
                detected,
                &request.target,
            )),
            Err(error) => Err(error),
        }
    }
}

pub fn temp_store() -> (TempDir, Arc<SettingsStore>) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("settings.json");
    (directory, Arc::new(SettingsStore::open(path)))
}
