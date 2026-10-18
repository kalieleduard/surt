mod classification;
mod environment;
mod initialization;
mod invocation;
mod rules;
mod supervision;
mod transcript;

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;

use super::{
    AgentRuntime, FailureClass, RuntimeCompletion, RuntimeError, RuntimeEvent, RuntimeEventSink,
    RuntimeRequest, StopSignal,
};
use invocation::Invocation;
use transcript::{Observation, Transcript};

const STOP_GRACE_PERIOD: Duration = Duration::from_secs(10);

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
    ) -> ClaudeCodeRuntime {
        ClaudeCodeRuntime {
            executable: executable.into(),
            environment: environment.into_iter().collect(),
            stop_grace_period: STOP_GRACE_PERIOD,
        }
    }

    #[cfg(all(test, unix, feature = "test-support"))]
    pub(crate) fn with_stop_grace_period(self, stop_grace_period: Duration) -> ClaudeCodeRuntime {
        ClaudeCodeRuntime {
            stop_grace_period,
            ..self
        }
    }
}

impl AgentRuntime for ClaudeCodeRuntime {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let invocation = Invocation::assemble(&request, &self.environment)?;
        write_launch_files(&invocation)?;
        let mut child = supervision::spawn(&self.executable, &invocation)?;
        let _ = events.send(RuntimeEvent::Launched { pid: child.id() });
        let stdin = child.stdin().take();
        let stdout = child.stdout().take();
        let stderr = child.stderr().take();
        let incompatible = StopSignal::new();
        let transcript = Transcript::new(invocation.expectation);
        let (transcript, (), (), exit) = tokio::join!(
            follow(stdout, transcript, &events, &incompatible),
            feed(stdin, &invocation.stdin),
            drain(stderr),
            supervision::supervise(child, &stop, &incompatible, self.stop_grace_period),
        );
        transcript.conclude(exit?)
    }
}

fn write_launch_files(invocation: &Invocation) -> Result<(), RuntimeError> {
    invocation.files.iter().try_for_each(|(path, contents)| {
        path.parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(path, contents))
            .map_err(|error| {
                RuntimeError::new(
                    FailureClass::Retryable,
                    "launch_files",
                    format!("cannot write {}: {error}", path.display()),
                )
            })
    })
}

async fn follow(
    stdout: Option<impl AsyncRead + Unpin>,
    mut transcript: Transcript,
    events: &RuntimeEventSink,
    incompatible: &StopSignal,
) -> Transcript {
    let Some(stdout) = stdout else {
        return transcript;
    };
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    while matches!(reader.read_until(b'\n', &mut line).await, Ok(read) if read > 0) {
        match transcript.observe(&line) {
            Observation::Initialized(initialization) => {
                let _ = events.send(RuntimeEvent::Initialized(initialization));
            }
            Observation::Incompatible => incompatible.force(),
            Observation::Ignored => {}
        }
        line.clear();
    }
    transcript
}

async fn feed(stdin: Option<ChildStdin>, context: &str) {
    if let Some(mut stdin) = stdin {
        let _ = stdin.write_all(context.as_bytes()).await;
    }
}

async fn drain(stderr: Option<impl AsyncRead + Unpin>) {
    if let Some(mut stderr) = stderr {
        let _ = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
