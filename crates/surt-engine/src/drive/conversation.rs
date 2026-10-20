use uuid::Uuid;

use super::driver::Driver;
use super::frozen::FrozenAgent;
use super::load::label;
use super::outcome::Progress;
use crate::context::Assembly;
use crate::project::Error;
use crate::runtime::{AgentRuntime, Continuation, Conversation};
use crate::store::{PreviousConversation, StartedExecution, StepRow};
use crate::workflow::ConversationScope;

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn continuation(
        &self,
        row: &StepRow,
        agent: &FrozenAgent,
        started: &StartedExecution,
        assembly: &Assembly<'_>,
    ) -> Result<Option<Continuation>, Error> {
        if agent.contract.profile.conversation != ConversationScope::Loop {
            return Ok(None);
        }
        let Some(previous) = self.driving.store.previous_conversation(&row.id).await? else {
            return Ok(None);
        };
        let Ok(session_id) = Uuid::parse_str(&previous.session_id) else {
            return Ok(None);
        };
        let context = assembly
            .write_continued(&changed(&started.inputs, &previous))
            .map_err(|error| Error::io("write the context in", assembly.execution_dir, error))?;
        Ok(Some(Continuation {
            session_id,
            execution_id: previous.execution,
            context,
        }))
    }

    pub(super) fn report_restart(&self, row: &StepRow, conversation: Option<&Conversation>) {
        if let Some(Conversation::Restarted { reason, .. }) = conversation {
            self.report(Progress::ConversationRestarted {
                step: label(row),
                reason: *reason,
            });
        }
    }
}

fn changed<'a>(inputs: &'a [(String, String)], previous: &PreviousConversation) -> Vec<&'a str> {
    inputs
        .iter()
        .filter(|(name, revision)| {
            previous.inputs.get(name) != Some(revision) && !previous.produced.contains(revision)
        })
        .map(|(name, _)| name.as_str())
        .collect()
}
