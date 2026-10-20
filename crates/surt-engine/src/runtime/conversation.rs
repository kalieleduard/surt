use serde_json::{Value, json};
use uuid::Uuid;

use super::RuntimeRequest;
use crate::workflow::ConversationScope;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Continuation {
    pub(crate) session_id: Uuid,
    pub(crate) execution_id: String,
    pub(crate) context: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conversation {
    ContinuedFrom(String),
    Restarted {
        reason: RestartReason,
        previous: String,
    },
    Fresh(FreshReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartReason {
    TranscriptMissing,
    ContinuationRefused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreshReason {
    NoPrevious,
}

impl RuntimeRequest {
    pub(crate) fn continued(&self) -> Option<Conversation> {
        match &self.continuation {
            Some(continuation) => Some(Conversation::ContinuedFrom(
                continuation.execution_id.clone(),
            )),
            None => self.fresh(),
        }
    }

    pub(crate) fn fresh(&self) -> Option<Conversation> {
        (self.launch.profile.conversation == ConversationScope::Loop)
            .then_some(Conversation::Fresh(FreshReason::NoPrevious))
    }
}

impl Conversation {
    pub(crate) fn value(&self) -> Value {
        match self {
            Conversation::ContinuedFrom(execution) => json!({ "continued_from": execution }),
            Conversation::Restarted { reason, previous } => {
                json!({ "restarted": reason.name(), "previous": previous })
            }
            Conversation::Fresh(reason) => json!({ "fresh": reason.name() }),
        }
    }
}

impl RestartReason {
    fn name(self) -> &'static str {
        match self {
            RestartReason::TranscriptMissing => "transcript_missing",
            RestartReason::ContinuationRefused => "continuation_refused",
        }
    }
}

impl FreshReason {
    fn name(self) -> &'static str {
        match self {
            FreshReason::NoPrevious => "no_previous",
        }
    }
}
