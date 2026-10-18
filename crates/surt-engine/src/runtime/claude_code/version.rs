use std::ffi::OsString;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use super::supervision::launch_failure;
use crate::runtime::{FailureClass, RuntimeError};

const MINIMUM: [u32; 3] = [2, 1, 301];
pub(super) const MINIMUM_TEXT: &str = "2.1.301";
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
const VERSION_FLAG: &str = "--version";

pub(super) async fn probe(
    executable: &Path,
    environment: &[(OsString, OsString)],
) -> Result<String, RuntimeError> {
    let output = Command::new(executable)
        .arg(VERSION_FLAG)
        .env_clear()
        .envs(environment.iter().cloned())
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .output();
    let output = tokio::time::timeout(PROBE_TIMEOUT, output)
        .await
        .map_err(|_| unreadable(executable, "it did not answer in time"))?
        .map_err(|error| launch_failure(executable, &error))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let reported = stdout.split_whitespace().next().unwrap_or_default();
    if !output.status.success() || parse(reported).is_none() {
        return Err(unreadable(
            executable,
            &format!("it answered {:?} ({})", stdout.trim(), output.status),
        ));
    }
    Ok(reported.to_owned())
}

pub(super) fn require_supported(reported: &str) -> Result<(), RuntimeError> {
    match parse(reported) {
        Some(version) if version >= MINIMUM => Ok(()),
        _ => Err(RuntimeError::new(
            FailureClass::NotRetryable,
            "unsupported_version",
            format!(
                "Claude Code {reported} is older than the supported {MINIMUM_TEXT}; \
                 upgrade it with `claude update`"
            ),
        )),
    }
}

fn parse(text: &str) -> Option<[u32; 3]> {
    let core = text.split(['-', '+', ' ']).next()?;
    let mut parts = core.split('.').map(|part| part.parse::<u32>().ok());
    let version = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(version)
}

fn unreadable(executable: &Path, detail: &str) -> RuntimeError {
    RuntimeError::new(
        FailureClass::NotRetryable,
        "unsupported_version",
        format!(
            "cannot read a Claude Code version from `{} {VERSION_FLAG}`: {detail}; \
             install Claude Code {MINIMUM_TEXT} or newer",
            executable.display()
        ),
    )
}
