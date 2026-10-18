use super::Stop;

pub(super) fn signal(pid: u32, stop: Stop) {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return;
    };
    let signal = match stop {
        Stop::Graceful => libc::SIGTERM,
        Stop::Forced => libc::SIGKILL,
    };
    if unsafe { libc::kill(-pid, signal) } != 0 {
        unsafe { libc::kill(pid, signal) };
    }
}
