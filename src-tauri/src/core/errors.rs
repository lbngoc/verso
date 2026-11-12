use serde::{Deserialize, Serialize};
use std::fmt;

/// Every failure an engine (or the pipeline in front of them) can report.
///
/// `EmptyInput` is raised by the pipeline before any engine runs (data-model § TranslationRequest);
/// it exists here so adapters can enforce the same rule when called directly in contract tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EngineError {
    #[error("no connection")]
    NoConnection,
    #[error("the translation service took too long")]
    Timeout,
    #[error("the translation service is busy (too many requests)")]
    RateLimited,
    /// The service refused requests from this network (a CAPTCHA page or a 401/403 that a fresh
    /// session does not clear). Unlike `RateLimited` it is not worth an immediate retry.
    #[error("the translation service refused requests from this network")]
    Blocked,
    #[error("the engine key is not valid")]
    InvalidKey,
    #[error("the language pair is not supported")]
    UnsupportedPair,
    #[error("the engine is not installed")]
    NotInstalled,
    #[error("the answer could not be read")]
    ParseError,
    #[error("there is nothing to translate")]
    EmptyInput,
    #[error("internal error: {message}")]
    Internal { message: String },
}

impl EngineError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Failures after which the next engine should be tried (engine-contract § Pipeline rules 3).
    pub fn is_failover(&self) -> bool {
        matches!(
            self,
            Self::NoConnection
                | Self::Timeout
                | Self::RateLimited
                | Self::Blocked
                | Self::ParseError
                | Self::NotInstalled
                | Self::InvalidKey
                | Self::UnsupportedPair
                | Self::Internal { .. }
        )
    }

    /// Starts a 60 s cooldown for the engine (data-model § Engine state).
    pub fn starts_cooldown(&self) -> bool {
        matches!(
            self,
            Self::NoConnection | Self::Timeout | Self::RateLimited | Self::Blocked
        )
    }

    /// Lower ranks are more actionable for the user (FR-008).
    fn actionability(&self) -> u8 {
        match self {
            Self::NoConnection => 0,
            Self::Blocked => 1,
            Self::RateLimited => 2,
            Self::Timeout => 3,
            Self::InvalidKey => 4,
            Self::UnsupportedPair => 5,
            Self::NotInstalled => 6,
            Self::ParseError => 7,
            Self::EmptyInput => 8,
            Self::Internal { .. } => 9,
        }
    }

    /// Picks the most actionable cause from a full round of engine failures.
    pub fn most_actionable(errors: impl IntoIterator<Item = EngineError>) -> Option<Self> {
        errors.into_iter().min_by_key(|e| e.actionability())
    }
}

impl From<reqwest::Error> for EngineError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            EngineError::Timeout
        } else if err.is_connect() || err.is_request() {
            EngineError::NoConnection
        } else if err.is_decode() {
            EngineError::ParseError
        } else {
            EngineError::Internal {
                message: short_reqwest_message(&err),
            }
        }
    }
}

fn short_reqwest_message(err: &reqwest::Error) -> String {
    // Never carry a URL (the Google adapter puts the text in the query string).
    if err.is_body() {
        "response body error".into()
    } else if err.is_status() {
        "unexpected status".into()
    } else {
        "request failed".into()
    }
}

impl fmt::Display for AttemptOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttemptOutcome::Ok => write!(f, "ok"),
            AttemptOutcome::Error { error } => write!(f, "{error}"),
        }
    }
}

/// Outcome of one engine call, recorded for the engine label and `engine-fallback` (FR-005/FR-008).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AttemptOutcome {
    Ok,
    Error { error: EngineError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineAttempt {
    pub engine_id: String,
    pub outcome: AttemptOutcome,
    pub latency_ms: u64,
}

impl EngineAttempt {
    pub fn ok(engine_id: impl Into<String>, latency_ms: u64) -> Self {
        Self {
            engine_id: engine_id.into(),
            outcome: AttemptOutcome::Ok,
            latency_ms,
        }
    }

    pub fn error(engine_id: impl Into<String>, error: EngineError, latency_ms: u64) -> Self {
        Self {
            engine_id: engine_id.into(),
            outcome: AttemptOutcome::Error { error },
            latency_ms,
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self.outcome, AttemptOutcome::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blocked_network_outranks_a_busy_service() {
        // "Refused from this network" tells the user more than "busy, try again".
        let picked = EngineError::most_actionable([EngineError::RateLimited, EngineError::Blocked]);
        assert_eq!(picked, Some(EngineError::Blocked));
    }

    #[test]
    fn blocked_moves_on_and_cools_down() {
        assert!(EngineError::Blocked.is_failover());
        assert!(EngineError::Blocked.starts_cooldown());
    }
}
