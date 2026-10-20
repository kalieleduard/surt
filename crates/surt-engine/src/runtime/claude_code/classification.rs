use serde_json::Value;

use super::result::ResultEvent;
use crate::runtime::{FailureClass, RuntimeError};

const ALLOWED_RATE: &str = "allowed";
const API_ERROR: &str = "api_error";
const RATE_LIMIT: &str = "rate_limit";
const UNKNOWN: &str = "unknown";
const NETWORK: &str = "network";

const NOT_RETRYABLE: &[&str] = &[
    "authentication_failed",
    "oauth_org_not_allowed",
    "account_on_hold",
    "billing_error",
    "cloud_credential_error",
    "model_not_found",
    "invalid_request",
];
const TRANSIENT: &[&str] = &[RATE_LIMIT, "overloaded", "server_error"];

#[derive(Debug, Default)]
pub(super) struct Signals {
    assistant_error: Option<String>,
    assistant_text: Option<String>,
    rate_limit_reset: Option<i64>,
    transport_retry: bool,
}

impl Signals {
    pub(super) fn observe(&mut self, event: &Value) {
        match (event["type"].as_str(), event["subtype"].as_str()) {
            (Some("assistant"), _) => self.assistant(event),
            (Some("rate_limit_event"), _) => self.rate_limit(&event["rate_limit_info"]),
            (Some("system"), Some("api_retry")) => {
                self.transport_retry = event["error_status"].is_null();
            }
            _ => {}
        }
    }

    pub(super) fn classify(&self, result: Option<&ResultEvent>) -> Option<RuntimeError> {
        if result.is_some_and(ResultEvent::succeeded) {
            return None;
        }
        let message = result
            .and_then(|result| result.text.clone())
            .or_else(|| self.assistant_text.clone());
        let category = self.assistant_error.as_deref();
        let api_failure =
            result.is_some_and(|result| result.terminal_reason.as_deref() == Some(API_ERROR));
        let lost_transport = self.transport_retry
            && category.is_none_or(|category| category == UNKNOWN)
            && (api_failure || result.is_none());
        if lost_transport {
            return Some(failure(FailureClass::Transient, NETWORK, message));
        }
        if let Some(category) = category {
            let error = failure(class_of(category), category, message);
            return Some(match category {
                RATE_LIMIT => error.resetting_at(self.rate_limit_reset),
                _ => error,
            });
        }
        result.map(|result| failure(FailureClass::Retryable, result.failure_code(), message))
    }

    fn assistant(&mut self, event: &Value) {
        let Some(error) = event["error"].as_str() else {
            return;
        };
        self.assistant_error = Some(error.to_owned());
        self.assistant_text = event["message"]["content"]
            .as_array()
            .and_then(|content| content.iter().find_map(|block| block["text"].as_str()))
            .map(str::to_owned);
    }

    fn rate_limit(&mut self, info: &Value) {
        if info["status"] == ALLOWED_RATE {
            return;
        }
        if let Some(resets_at) = info["resetsAt"].as_i64() {
            self.rate_limit_reset = Some(resets_at);
        }
    }
}

fn class_of(category: &str) -> FailureClass {
    if NOT_RETRYABLE.contains(&category) {
        FailureClass::NotRetryable
    } else if TRANSIENT.contains(&category) {
        FailureClass::Transient
    } else {
        FailureClass::Retryable
    }
}

fn failure(class: FailureClass, code: &str, message: Option<String>) -> RuntimeError {
    let message = message.unwrap_or_else(|| format!("Claude Code reported {code}"));
    RuntimeError::new(class, code, message)
}
