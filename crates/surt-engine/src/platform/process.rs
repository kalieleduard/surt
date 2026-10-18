#[cfg_attr(target_os = "linux", path = "process/linux.rs")]
#[cfg_attr(target_os = "macos", path = "process/macos.rs")]
#[cfg_attr(windows, path = "process/windows.rs")]
mod native;
#[cfg(unix)]
mod signals;

use std::time::Duration;

use tokio::time::{Instant, sleep};

const STOP_POLL: Duration = Duration::from_millis(20);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProcessIdentity {
    pub(crate) pid: u32,
    pub(crate) start_time: i64,
    pub(crate) boot_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stop {
    Graceful,
    Forced,
}

pub(crate) fn identify(pid: u32) -> Option<ProcessIdentity> {
    Some(ProcessIdentity {
        pid,
        start_time: native::start_time(pid)?,
        boot_id: native::boot_id()?,
    })
}

pub(crate) fn is_alive(identity: &ProcessIdentity) -> bool {
    identify(identity.pid).as_ref() == Some(identity)
}

pub(crate) async fn stop(identity: &ProcessIdentity, grace: Duration) -> bool {
    if !is_alive(identity) {
        return false;
    }
    native::signal(identity.pid, Stop::Graceful);
    if !gone_within(identity, grace).await && is_alive(identity) {
        native::signal(identity.pid, Stop::Forced);
        gone_within(identity, grace).await;
    }
    true
}

async fn gone_within(identity: &ProcessIdentity, period: Duration) -> bool {
    let deadline = Instant::now() + period;
    while is_alive(identity) {
        if Instant::now() >= deadline {
            return false;
        }
        sleep(STOP_POLL).await;
    }
    true
}

#[cfg(test)]
mod tests;
