mod tail;

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

#[cfg(windows)]
use process_wrap::tokio::JobObject;
#[cfg(unix)]
use process_wrap::tokio::ProcessGroup;
use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;

pub const REPORT_SCHEMA: (&str, &str) = (
    "schemas/check-report.schema.json",
    include_str!("../assets/schemas/check-report.schema.json"),
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckSpec {
    pub(crate) name: String,
    pub(crate) run: Vec<String>,
    pub(crate) timeout: Option<Duration>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CheckReport {
    commit: String,
    checks: Vec<CheckResult>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CheckResult {
    name: String,
    status: Status,
    exit_code: Option<i32>,
    duration_ms: u64,
    output_tail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Passed,
    Failed,
    TimedOut,
    NotStarted,
}

enum Ended {
    Exited(ExitStatus),
    TimedOut,
    NotStarted(io::Error),
}

impl CheckReport {
    pub(crate) fn checks(&self) -> &[CheckResult] {
        &self.checks
    }

    pub(crate) fn passed(&self) -> bool {
        self.checks
            .iter()
            .all(|check| check.status == Status::Passed)
    }
}

pub(crate) async fn run(
    checks: &[CheckSpec],
    worktree: &Path,
    commit: &str,
    log: &Path,
    spawned: &UnboundedSender<u32>,
) -> io::Result<CheckReport> {
    if let Some(dir) = log.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut output = OpenOptions::new().create(true).append(true).open(log)?;
    let mut results = Vec::with_capacity(checks.len());
    for check in checks {
        results.push(run_one(check, worktree, &mut output, log, spawned).await?);
    }
    Ok(CheckReport {
        commit: commit.to_owned(),
        checks: results,
    })
}

async fn run_one(
    check: &CheckSpec,
    worktree: &Path,
    output: &mut File,
    log: &Path,
    spawned: &UnboundedSender<u32>,
) -> io::Result<CheckResult> {
    writeln!(output, "=== {}: {}", check.name, check.run.join(" "))?;
    let offset = output.metadata()?.len();
    let started = Instant::now();
    let ended = match spawn(check, worktree, output) {
        Ok(child) => {
            if let Some(pid) = child.id() {
                let _ = spawned.send(pid);
            }
            supervise(child, check.timeout).await?
        }
        Err(error) => Ended::NotStarted(error),
    };
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let (status, exit_code) = match ended {
        Ended::Exited(status) if status.success() => (Status::Passed, status.code()),
        Ended::Exited(status) => (Status::Failed, status.code()),
        Ended::TimedOut => {
            writeln!(output, "surt: {} exceeded its timeout", check.name)?;
            (Status::TimedOut, None)
        }
        Ended::NotStarted(error) => {
            writeln!(output, "surt: cannot run {}: {error}", check.run[0])?;
            (Status::NotStarted, None)
        }
    };
    Ok(CheckResult {
        name: check.name.clone(),
        status,
        exit_code,
        duration_ms,
        output_tail: tail::read(log, offset)?,
    })
}

fn spawn(check: &CheckSpec, worktree: &Path, output: &File) -> io::Result<Box<dyn ChildWrapper>> {
    let stdout = output.try_clone()?;
    let stderr = output.try_clone()?;
    let mut command = CommandWrap::with_new(&check.run[0], |command| {
        command
            .args(&check.run[1..])
            .current_dir(worktree)
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr);
    });
    #[cfg(unix)]
    command.wrap(ProcessGroup::leader());
    #[cfg(windows)]
    command.wrap(JobObject);
    command.wrap(KillOnDrop);
    command.spawn()
}

async fn supervise(
    mut child: Box<dyn ChildWrapper>,
    timeout: Option<Duration>,
) -> io::Result<Ended> {
    let Some(timeout) = timeout else {
        return Ok(Ended::Exited(child.wait().await?));
    };
    if let Ok(status) = tokio::time::timeout(timeout, child.wait()).await {
        return Ok(Ended::Exited(status?));
    }
    child.start_kill()?;
    child.wait().await?;
    Ok(Ended::TimedOut)
}
