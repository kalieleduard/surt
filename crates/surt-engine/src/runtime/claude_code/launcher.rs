use std::fs;
use std::path::Path;
use std::time::Duration;

use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;

use super::invocation::Invocation;
use super::supervision;
use super::transcript::{Observation, Transcript};
use crate::runtime::{
    Exit, FailureClass, RuntimeError, RuntimeEvent, RuntimeEventSink, STDERR_LOG, STREAM_LOG,
    StopSignal,
};

pub(super) struct Launcher<'a> {
    pub(super) executable: &'a Path,
    pub(super) log_dir: &'a Path,
    pub(super) events: &'a RuntimeEventSink,
    pub(super) stop: &'a StopSignal,
    pub(super) grace: Duration,
}

#[derive(Debug)]
pub(super) struct Launched {
    pub(super) transcript: Transcript,
    pub(super) exit: Exit,
}

impl Launcher<'_> {
    pub(super) async fn launch(&self, invocation: Invocation) -> Result<Launched, RuntimeError> {
        write_launch_files(&invocation)?;
        let stream_log = open_log(self.log_dir, STREAM_LOG).await?;
        let stderr_log = open_log(self.log_dir, STDERR_LOG).await?;
        let mut child = supervision::spawn(self.executable, &invocation)?;
        let _ = self.events.send(RuntimeEvent::Launched { pid: child.id() });
        let stdin = child.stdin().take();
        let stdout = child.stdout().take();
        let stderr = child.stderr().take();
        let incompatible = StopSignal::new();
        let follower = Follower {
            transcript: Transcript::new(invocation.expectation),
            log: stream_log,
            events: self.events,
            incompatible: &incompatible,
        };
        let (transcript, (), (), exit) = tokio::join!(
            follower.follow(stdout),
            feed(stdin, &invocation.stdin),
            drain(stderr, stderr_log),
            supervision::supervise(child, self.stop, &incompatible, self.grace),
        );
        Ok(Launched {
            transcript,
            exit: exit?,
        })
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
