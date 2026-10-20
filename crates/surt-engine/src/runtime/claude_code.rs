mod classification;
mod envelope;
mod environment;
mod initialization;
mod invocation;
mod launcher;
mod report;
mod restart;
mod result;
mod rules;
mod supervision;
mod transcript;
mod version;

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use super::{
    AgentRuntime, Conversation, RuntimeCompletion, RuntimeError, RuntimeEvent, RuntimeEventSink,
    RuntimeRequest, StopSignal,
};
use crate::platform;
use invocation::Invocation;
use launcher::{Launched, Launcher};

#[derive(Clone, Debug)]
pub struct ClaudeCodeRuntime {
    executable: PathBuf,
    environment: Vec<(OsString, OsString)>,
    stop_grace_period: Duration,
}

impl ClaudeCodeRuntime {
    pub fn new(
        executable: impl Into<PathBuf>,
        environment: impl IntoIterator<Item = (OsString, OsString)>,
        stop_grace_period: Duration,
    ) -> ClaudeCodeRuntime {
        ClaudeCodeRuntime {
            executable: executable.into(),
            environment: environment.into_iter().collect(),
            stop_grace_period,
        }
    }

    async fn converse(
        &self,
        request: &RuntimeRequest,
        invocation: Invocation,
        launcher: &Launcher<'_>,
    ) -> Result<(Launched, Option<Conversation>), RuntimeError> {
        let launched = launcher.launch(invocation).await?;
        let Some(restarted) = restart::needed(request, &launched, launcher.stop) else {
            return Ok((launched, request.continued()));
        };
        let fresh = Invocation::assemble(&restart::fresh(request), &self.environment)?;
        Ok((launcher.launch(fresh).await?, Some(restarted)))
    }

    async fn preflight(
        &self,
        request: &RuntimeRequest,
        events: &RuntimeEventSink,
    ) -> Result<(Invocation, PathBuf), RuntimeError> {
        let invocation = Invocation::assemble(request, &self.environment)?;
        supervision::require_working_dir(&invocation)?;
        let executable = platform::find_executable(&self.executable, &invocation.environment);
        let version = version::probe(&executable, &invocation.environment).await?;
        let _ = events.send(RuntimeEvent::Identified {
            version: version.clone(),
        });
        version::require_supported(&version)?;
        Ok((invocation, executable))
    }
}

impl AgentRuntime for ClaudeCodeRuntime {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let (invocation, executable) = self.preflight(&request, &events).await?;
        let launcher = Launcher {
            executable: &executable,
            log_dir: &request.log_dir,
            events: &events,
            stop: &stop,
            grace: self.stop_grace_period,
        };
        let (launched, conversation) = self.converse(&request, invocation, &launcher).await?;
        let mut report = launched.transcript.report(launched.exit);
        report.conversation = conversation;
        let _ = events.send(RuntimeEvent::Reported(report));
        launched.transcript.conclude(launched.exit)
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
