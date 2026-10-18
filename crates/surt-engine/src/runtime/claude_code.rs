mod classification;
mod environment;
mod initialization;
mod invocation;
mod rules;
mod supervision;
mod transcript;
mod version;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;

use super::{
    AgentRuntime, FailureClass, RuntimeCompletion, RuntimeError, RuntimeEvent, RuntimeEventSink,
    RuntimeRequest, StopSignal,
};
use crate::platform;
use invocation::Invocation;
use transcript::{Observation, Transcript};

const STOP_GRACE_PERIOD: Duration = Duration::from_secs(10);
const STREAM_LOG: &str = "stream.jsonl";
const STDERR_LOG: &str = "stderr.log";

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
        supervision::require_working_dir(&invocation)?;
        let executable = platform::find_executable(&self.executable, &invocation.environment);
        let version = version::probe(&executable, &invocation.environment).await?;
        let _ = events.send(RuntimeEvent::Identified {
            version: version.clone(),
        });
        version::require_supported(&version)?;
        write_launch_files(&invocation)?;
        let stream_log = open_log(&request.log_dir, STREAM_LOG).await?;
        let stderr_log = open_log(&request.log_dir, STDERR_LOG).await?;
        let mut child = supervision::spawn(&executable, &invocation)?;
        let _ = events.send(RuntimeEvent::Launched { pid: child.id() });
        let stdin = child.stdin().take();
        let stdout = child.stdout().take();
        let stderr = child.stderr().take();
        let incompatible = StopSignal::new();
        let follower = Follower {
            transcript: Transcript::new(invocation.expectation),
            log: stream_log,
            events: &events,
            incompatible: &incompatible,
        };
        let (transcript, (), (), exit) = tokio::join!(
            follower.follow(stdout),
            feed(stdin, &invocation.stdin),
            drain(stderr, stderr_log),
            supervision::supervise(child, &stop, &incompatible, self.stop_grace_period),
        );
        transcript.conclude(exit?)
    }
}

struct Follower<'a> {
    transcript: Transcript,
    log: File,
    events: &'a RuntimeEventSink,
    incompatible: &'a StopSignal,
}

impl Follower<'_> {
    async fn follow(mut self, stdout: Option<impl AsyncRead + Unpin>) -> Transcript {
        let Some(stdout) = stdout else {
            return self.transcript;
        };
        let mut reader = BufReader::new(stdout);
        let mut line = Vec::new();
        while matches!(reader.read_until(b'\n', &mut line).await, Ok(read) if read > 0) {
            let _ = self.log.write_all(&line).await;
            match self.transcript.observe(&line) {
                Observation::Initialized(initialization) => {
                    let _ = self.events.send(RuntimeEvent::Initialized(initialization));
                }
                Observation::Incompatible => self.incompatible.force(),
                Observation::Ignored => {}
            }
            line.clear();
        }
        let _ = self.log.flush().await;
        self.transcript
    }
}

async fn open_log(log_dir: &Path, name: &str) -> Result<File, RuntimeError> {
    let path = log_dir.join(name);
    let opened = async {
        tokio::fs::create_dir_all(log_dir).await?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
    };
    opened.await.map_err(|error| {
        RuntimeError::new(
            FailureClass::Retryable,
            "log_files",
            format!("cannot open {}: {error}", path.display()),
        )
    })
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

async fn feed(stdin: Option<ChildStdin>, context: &str) {
    if let Some(mut stdin) = stdin {
        let _ = stdin.write_all(context.as_bytes()).await;
    }
}

async fn drain(stderr: Option<impl AsyncRead + Unpin>, mut log: File) {
    if let Some(mut stderr) = stderr {
        let _ = tokio::io::copy(&mut stderr, &mut log).await;
        let _ = log.flush().await;
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
