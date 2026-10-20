use serde_json::Value;

use super::classification::Signals;
use super::invocation::Expectation;
use super::result::ResultEvent;
use super::{initialization, report};
use crate::runtime::{
    Exit, FailureClass, Initialization, RestartReason, RuntimeCompletion, RuntimeError,
    RuntimeReport,
};

#[derive(Debug)]
pub(super) struct Transcript {
    expectation: Expectation,
    initialization: Option<Initialization>,
    incompatibility: Option<RuntimeError>,
    result: Option<ResultEvent>,
    protocol_error: Option<RuntimeError>,
    signals: Signals,
    report: RuntimeReport,
}

#[derive(Debug, PartialEq)]
pub(super) enum Observation {
    Ignored,
    Initialized(Initialization),
    Incompatible,
}

impl Transcript {
    pub(super) fn new(expectation: Expectation) -> Transcript {
        Transcript {
            report: report::requested(expectation.effort),
            expectation,
            initialization: None,
            incompatibility: None,
            result: None,
            protocol_error: None,
            signals: Signals::default(),
        }
    }

    pub(super) fn observe(&mut self, line: &[u8]) -> Observation {
        let line = line.trim_ascii();
        if line.is_empty() {
            return Observation::Ignored;
        }
        let Ok(event) = serde_json::from_slice::<Value>(line) else {
            self.protocol_failure("the stream carried a line that is not JSON");
            return Observation::Ignored;
        };
        match (event["type"].as_str(), event["subtype"].as_str()) {
            (Some("system"), Some("init")) => self.initialize(&event),
            (Some("result"), _) => {
                self.finish(&event);
                Observation::Ignored
            }
            _ => {
                self.signals.observe(&event);
                Observation::Ignored
            }
        }
    }

    pub(super) fn report(&self, exit: Exit) -> RuntimeReport {
        RuntimeReport {
            exit_code: exit.code(),
            ..self.report.clone()
        }
    }

    pub(super) fn conclude(self, exit: Exit) -> Result<RuntimeCompletion, RuntimeError> {
        if exit != Exit::Stopped
            && let Some(failure) = self.failure()
        {
            return Err(failure);
        }
        Ok(RuntimeCompletion {
            exit,
            initialization: self.initialization,
            result: self.result.map(ResultEvent::evidence),
        })
    }

    pub(super) fn refused_continuation(&self) -> Option<RestartReason> {
        if self.initialization.is_some() || self.incompatibility.is_some() {
            return None;
        }
        let missing = self
            .result
            .as_ref()
            .is_some_and(ResultEvent::lost_conversation);
        Some(if missing {
            RestartReason::TranscriptMissing
        } else {
            RestartReason::ContinuationRefused
        })
    }

    fn initialize(&mut self, event: &Value) -> Observation {
        if self.initialization.is_some() || self.incompatibility.is_some() {
            return Observation::Ignored;
        }
        report::initialized(&mut self.report, event);
        match initialization::check(&self.expectation, event) {
            Ok(initialization) => {
                self.initialization = Some(initialization.clone());
                Observation::Initialized(initialization)
            }
            Err(incompatibility) => {
                self.incompatibility = Some(incompatibility);
                Observation::Incompatible
            }
        }
    }

    fn finish(&mut self, event: &Value) {
        if self.result.is_some() {
            self.protocol_failure("the stream carried more than one result");
            return;
        }
        report::finished(&mut self.report, event);
        self.result = Some(ResultEvent::parse(event));
    }

    fn protocol_failure(&mut self, message: &str) {
        self.protocol_error.get_or_insert_with(|| {
            RuntimeError::new(FailureClass::Retryable, "protocol_error", message)
        });
    }

    fn failure(&self) -> Option<RuntimeError> {
        self.incompatibility
            .clone()
            .or_else(|| self.protocol_error.clone())
            .or_else(|| self.signals.classify(self.result.as_ref()))
    }
}
