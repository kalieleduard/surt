use std::io;
use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

#[cfg(windows)]
use process_wrap::tokio::JobObject;
#[cfg(unix)]
use process_wrap::tokio::ProcessGroup;
use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use tokio::time::{Instant, sleep_until};

use super::invocation::Invocation;
use crate::runtime::{Exit, FailureClass, RuntimeError, StopLevel, StopSignal};

#[cfg(unix)]
const SIGTERM: i32 = 15;
#[cfg(unix)]
const SIGNALLED_EXIT_BASE: i32 = 128;
const UNKNOWN_EXIT_CODE: i32 = -1;

pub(super) fn spawn(
    executable: &Path,
    invocation: &Invocation,
) -> Result<Box<dyn ChildWrapper>, RuntimeError> {
    if !invocation.working_dir.is_dir() {
        return Err(RuntimeError::new(
            FailureClass::Retryable,
            "missing_working_dir",
            format!("{} is not a directory", invocation.working_dir.display()),
        ));
    }
    let mut command = CommandWrap::with_new(executable, |command| {
        command
            .args(&invocation.arguments)
            .env_clear()
            .envs(invocation.environment.iter().cloned())
            .current_dir(&invocation.working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    });
    #[cfg(unix)]
    command.wrap(ProcessGroup::leader());
    #[cfg(windows)]
    command.wrap(JobObject);
    command.wrap(KillOnDrop);
    command
        .spawn()
        .map_err(|error| launch_failure(executable, &error))
}

pub(super) async fn supervise(
    mut child: Box<dyn ChildWrapper>,
    stop: &StopSignal,
    incompatible: &StopSignal,
    grace: Duration,
) -> Result<Exit, RuntimeError> {
    let mut escalation = Escalation::default();
    let status = loop {
        let deadline = escalation.deadline.unwrap_or_else(Instant::now);
        tokio::select! {
            status = child.wait() => break status,
            level = stop.reached(escalation.next_level()), if !escalation.killed => {
                escalation.stop(child.as_mut(), level, grace);
            }
            _ = incompatible.requested(), if !escalation.killed => escalation.kill(child.as_mut()),
            () = sleep_until(deadline), if escalation.deadline.is_some() && !escalation.killed => {
                escalation.kill(child.as_mut());
            }
        }
    };
    escalation.sweep(child.as_mut());
    let status = status.map_err(|error| {
        RuntimeError::new(FailureClass::Retryable, "wait_failed", error.to_string())
    })?;
    if escalation.stopped {
        return Ok(Exit::Stopped);
    }
    Ok(Exit::Code(exit_code(status)))
}

#[derive(Default)]
struct Escalation {
    stopped: bool,
    killed: bool,
    deadline: Option<Instant>,
}

impl Escalation {
    fn next_level(&self) -> StopLevel {
        if self.stopped {
            StopLevel::Force
        } else {
            StopLevel::Graceful
        }
    }

    fn stop(&mut self, child: &mut dyn ChildWrapper, level: StopLevel, grace: Duration) {
        self.stopped = true;
        if level == StopLevel::Force {
            self.kill(child);
            return;
        }
        terminate(child);
        self.deadline = Some(Instant::now() + grace);
    }

    fn kill(&mut self, child: &mut dyn ChildWrapper) {
        self.killed = true;
        let _ = child.start_kill();
    }

    fn sweep(&self, child: &mut dyn ChildWrapper) {
        if self.stopped || self.killed {
            let _ = child.start_kill();
        }
    }
}

#[cfg(unix)]
fn terminate(child: &mut dyn ChildWrapper) {
    let _ = child.signal(SIGTERM);
}

#[cfg(not(unix))]
fn terminate(child: &mut dyn ChildWrapper) {
    let _ = child.start_kill();
}

#[cfg(unix)]
fn exit_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status
        .signal()
        .map(|signal| SIGNALLED_EXIT_BASE + signal)
        .or(status.code())
        .unwrap_or(UNKNOWN_EXIT_CODE)
}

#[cfg(not(unix))]
fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(UNKNOWN_EXIT_CODE)
}

fn launch_failure(executable: &Path, error: &io::Error) -> RuntimeError {
    let (class, code) = match error.kind() {
        io::ErrorKind::NotFound => (FailureClass::NotRetryable, "missing_executable"),
        _ => (FailureClass::Retryable, "launch_failed"),
    };
    RuntimeError::new(
        class,
        code,
        format!(
            "cannot run Claude Code at {}: {error}",
            executable.display()
        ),
    )
}
