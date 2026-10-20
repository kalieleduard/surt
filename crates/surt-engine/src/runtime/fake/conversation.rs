use serde_json::{Value, json};

use super::Behavior;
use crate::runtime::{Conversation, RestartReason, RuntimeRequest};

pub(super) struct Turn<'a> {
    stdin: &'a str,
    resumed: Option<String>,
    pub(super) conversation: Option<Conversation>,
}

impl Behavior {
    pub(super) fn turn<'a>(&self, request: &'a RuntimeRequest) -> Turn<'a> {
        match (&request.continuation, self) {
            (Some(continuation), Behavior::TranscriptMissing(_)) => Turn {
                stdin: &request.context,
                resumed: None,
                conversation: Some(Conversation::Restarted {
                    reason: RestartReason::TranscriptMissing,
                    previous: continuation.execution_id.clone(),
                }),
            },
            (Some(continuation), _) => Turn {
                stdin: &continuation.context,
                resumed: Some(continuation.session_id.to_string()),
                conversation: request.continued(),
            },
            (None, _) => Turn {
                stdin: &request.context,
                resumed: None,
                conversation: request.continued(),
            },
        }
    }
}

impl Turn<'_> {
    pub(super) fn line(&self) -> Value {
        json!({ "type": "fake", "resumed": self.resumed, "stdin": self.stdin })
    }
}
