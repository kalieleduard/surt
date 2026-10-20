use serde_json::Value;

use crate::runtime::FinalResult;

const SUCCESS_SUBTYPE: &str = "success";
const COMPLETED: &str = "completed";
const NO_CONVERSATION: &str = "No conversation found";

#[derive(Debug)]
pub(super) struct ResultEvent {
    subtype: Option<String>,
    is_error: Option<bool>,
    pub(super) terminal_reason: Option<String>,
    pub(super) text: Option<String>,
    structured_output: Option<String>,
    errors: Vec<String>,
}

impl ResultEvent {
    pub(super) fn parse(event: &Value) -> ResultEvent {
        let text = |field: &str| event[field].as_str().map(str::to_owned);
        ResultEvent {
            subtype: text("subtype"),
            is_error: event["is_error"].as_bool(),
            terminal_reason: text("terminal_reason"),
            text: text("result"),
            structured_output: event
                .get("structured_output")
                .filter(|output| !output.is_null())
                .map(Value::to_string),
            errors: event["errors"]
                .as_array()
                .map(|errors| {
                    errors
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    pub(super) fn lost_conversation(&self) -> bool {
        self.errors
            .iter()
            .any(|error| error.contains(NO_CONVERSATION))
    }

    pub(super) fn succeeded(&self) -> bool {
        self.subtype.as_deref() == Some(SUCCESS_SUBTYPE)
            && self.is_error == Some(false)
            && self
                .terminal_reason
                .as_deref()
                .is_none_or(|reason| reason == COMPLETED)
    }

    pub(super) fn failure_code(&self) -> &str {
        let reason = self
            .terminal_reason
            .as_deref()
            .filter(|reason| *reason != COMPLETED);
        let subtype = self
            .subtype
            .as_deref()
            .filter(|subtype| *subtype != SUCCESS_SUBTYPE);
        reason.or(subtype).unwrap_or("unsuccessful_result")
    }

    pub(super) fn evidence(self) -> FinalResult {
        FinalResult {
            succeeded: self.succeeded(),
            structured_output: self.structured_output,
        }
    }
}
