//! Engine registry: user order, disabled engines, cooldown, retry-once-on-429, total budget.
//!
//! Implements engine-contract § Pipeline rules 2–8. No Tauri types here so contract tests can
//! drive it directly.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{debug, info};
use parking_lot::Mutex;
use rand::Rng;

use super::engine::{EngineInfo, TranslateEngine, TranslationRequest, TranslationResult};
use super::errors::{EngineAttempt, EngineError};
use super::settings::{SettingsProfile, SettingsStore};

pub const TOTAL_BUDGET: Duration = Duration::from_secs(14);
pub const COOLDOWN: Duration = Duration::from_secs(60);
pub const RETRY_BACKOFF_MS: std::ops::RangeInclusive<u64> = 400..=800;

pub type FallbackSink = Arc<dyn Fn(&str, &str, &EngineError) + Send + Sync>;

pub struct Registry {
    engines: Vec<Arc<dyn TranslateEngine>>,
    settings: Arc<SettingsStore>,
    cooldowns: Mutex<HashMap<String, Instant>>,
    total_budget: Duration,
    backoff_ms: std::ops::RangeInclusive<u64>,
    fake_429: Vec<String>,
    fallback_sink: Mutex<Option<FallbackSink>>,
}

impl Registry {
    pub fn new(engines: Vec<Arc<dyn TranslateEngine>>, settings: Arc<SettingsStore>) -> Self {
        let fake_429 = if cfg!(debug_assertions) {
            std::env::var("VERSO_FAKE_429")
                .ok()
                .map(|value| {
                    value
                        .split(',')
                        .map(|id| id.trim().to_string())
                        .filter(|id| !id.is_empty())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Self {
            engines,
            settings,
            cooldowns: Mutex::new(HashMap::new()),
            total_budget: TOTAL_BUDGET,
            backoff_ms: RETRY_BACKOFF_MS,
            fake_429,
            fallback_sink: Mutex::new(None),
        }
    }

    pub fn with_budget(mut self, budget: Duration) -> Self {
        self.total_budget = budget;
        self
    }

    pub fn with_backoff_ms(mut self, range: std::ops::RangeInclusive<u64>) -> Self {
        self.backoff_ms = range;
        self
    }

    pub fn with_fake_429(mut self, engines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.fake_429 = engines.into_iter().map(Into::into).collect();
        self
    }

    pub fn set_fallback_sink(&self, sink: FallbackSink) {
        *self.fallback_sink.lock() = Some(sink);
    }

    pub fn clear_cooldowns(&self) {
        self.cooldowns.lock().clear();
    }

    pub fn total_budget(&self) -> Duration {
        self.total_budget
    }

    pub fn engine_ids(&self) -> Vec<&'static str> {
        self.engines.iter().map(|engine| engine.id()).collect()
    }

    pub fn engine_infos(&self) -> Vec<EngineInfo> {
        let settings = self.settings.get();
        let order = effective_order(&self.engines, &settings);
        let now = Instant::now();
        order
            .iter()
            .enumerate()
            .map(|(position, engine)| {
                let remaining = self
                    .cooldowns
                    .lock()
                    .get(engine.id())
                    .map(|until| until.saturating_duration_since(now))
                    .unwrap_or_default();
                EngineInfo {
                    id: engine.id().to_string(),
                    service: engine.service().to_string(),
                    availability: engine.availability(),
                    priority: position + 1,
                    cooldown_remaining_s: remaining.as_secs(),
                    enabled: settings.engine_enabled(engine.id()),
                    timeout_ms: engine.timeout().as_millis() as u64,
                }
            })
            .collect()
    }

    /// Runs the engines the user left on, in order, within one 14 s budget.
    pub async fn translate(
        &self,
        request: &TranslationRequest,
    ) -> Result<TranslationResult, EngineError> {
        let settings = self.settings.get();
        let candidates: Vec<Arc<dyn TranslateEngine>> = effective_order(&self.engines, &settings)
            .into_iter()
            .filter(|engine| settings.engine_enabled(engine.id()))
            .collect();

        if candidates.is_empty() {
            return Err(EngineError::internal("no translation engine is enabled"));
        }

        let deadline = Instant::now() + self.total_budget;
        let mut attempts: Vec<EngineAttempt> = Vec::new();
        let mut errors: Vec<EngineError> = Vec::new();

        for (index, engine) in candidates.iter().enumerate() {
            let id = engine.id();

            if let Some(until) = self.cooldowns.lock().get(id).copied() {
                let remaining = until.saturating_duration_since(Instant::now());
                if !remaining.is_zero() {
                    debug!("{id} is in cooldown for {} ms", remaining.as_millis());
                    attempts.push(EngineAttempt::error(id, EngineError::RateLimited, 0));
                    errors.push(EngineError::RateLimited);
                    continue;
                }
            }

            if !engine.availability().is_available() {
                attempts.push(EngineAttempt::error(id, EngineError::NotInstalled, 0));
                errors.push(EngineError::NotInstalled);
                continue;
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining < Duration::from_millis(100) {
                errors.push(EngineError::Timeout);
                break;
            }

            let timeout = engine.timeout().min(remaining);
            let started = Instant::now();
            let first = self.call(engine.as_ref(), request, timeout).await;
            let latency = started.elapsed().as_millis() as u64;

            let failure = match first {
                Ok(mut result) => {
                    attempts.push(EngineAttempt::ok(id, latency));
                    result.attempts = attempts;
                    return Ok(result);
                }
                Err(error) => {
                    info!("{id} failed: {error}");
                    attempts.push(EngineAttempt::error(id, error.clone(), latency));
                    error
                }
            };

            // 429: one retry after 400–800 ms (FR-008), still inside the total budget.
            let mut failure = Some(failure);
            if matches!(failure, Some(EngineError::RateLimited)) {
                let backoff =
                    Duration::from_millis(rand::rng().random_range(self.backoff_ms.clone()));
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining > backoff + Duration::from_millis(200) {
                    tokio::time::sleep(backoff).await;
                    let started = Instant::now();
                    let still_remaining = deadline.saturating_duration_since(Instant::now());
                    let retry_timeout = engine.timeout().min(still_remaining);
                    match self.call(engine.as_ref(), request, retry_timeout).await {
                        Ok(mut result) => {
                            attempts
                                .push(EngineAttempt::ok(id, started.elapsed().as_millis() as u64));
                            result.attempts = attempts;
                            return Ok(result);
                        }
                        Err(error) => {
                            info!("{id} retry failed: {error}");
                            attempts.push(EngineAttempt::error(
                                id,
                                error.clone(),
                                started.elapsed().as_millis() as u64,
                            ));
                            failure = Some(error);
                        }
                    }
                }
            }

            if let Some(error) = failure {
                if error.starts_cooldown() {
                    self.cooldowns
                        .lock()
                        .insert(id.to_string(), Instant::now() + COOLDOWN);
                }
                let next = candidates.get(index + 1);
                if let (Some(next), Some(sink)) = (next, self.fallback_sink.lock().as_ref()) {
                    sink(id, next.id(), &error);
                }
                errors.push(error);
            }
        }

        Err(EngineError::most_actionable(errors)
            .unwrap_or_else(|| EngineError::internal("all engines failed")))
    }

    async fn call(
        &self,
        engine: &dyn TranslateEngine,
        request: &TranslationRequest,
        timeout: Duration,
    ) -> Result<TranslationResult, EngineError> {
        if cfg!(debug_assertions) && self.fake_429.iter().any(|id| id == engine.id()) {
            debug!("VERSO_FAKE_429 forces a 429 from {}", engine.id());
            return Err(EngineError::RateLimited);
        }
        engine.translate(request, timeout).await
    }
}

/// Registered engines in the user's order, followed by any engine the order does not mention.
fn effective_order(
    engines: &[Arc<dyn TranslateEngine>],
    settings: &SettingsProfile,
) -> Vec<Arc<dyn TranslateEngine>> {
    let mut ordered: Vec<Arc<dyn TranslateEngine>> = Vec::new();
    for id in &settings.engine_order {
        if let Some(engine) = engines.iter().find(|engine| engine.id() == id) {
            if !ordered.iter().any(|known| known.id() == id) {
                ordered.push(engine.clone());
            }
        }
    }
    for engine in engines {
        if !ordered.iter().any(|known| known.id() == engine.id()) {
            ordered.push(engine.clone());
        }
    }
    ordered
}
