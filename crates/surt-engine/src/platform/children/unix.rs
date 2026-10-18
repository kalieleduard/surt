use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

use process_wrap::tokio::{ChildWrapper, CommandWrap, ProcessGroup};

const SIGNALLED_EXIT_BASE: i32 = 128;
const UNKNOWN_EXIT_CODE: i32 = -1;

pub(super) fn own_tree(command: &mut CommandWrap) {
    command.wrap(ProcessGroup::leader());
    end_with_surt(command.command_mut());
}

pub(crate) fn interrupt(child: &mut dyn ChildWrapper) {
    let _ = child.signal(libc::SIGTERM);
}

pub(crate) fn exit_code(status: ExitStatus) -> i32 {
    status
        .signal()
        .map(|signal| SIGNALLED_EXIT_BASE + signal)
        .or(status.code())
        .unwrap_or(UNKNOWN_EXIT_CODE)
}

#[cfg(target_os = "linux")]
fn end_with_surt(command: &mut tokio::process::Command) {
    use std::io;

    let surt = std::process::id();
    let follow_surt = move || -> io::Result<()> {
        if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM, 0, 0, 0) } != 0 {
            return Err(io::Error::last_os_error());
        }
        if u32::try_from(unsafe { libc::getppid() }).ok() == Some(surt) {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(libc::ESRCH))
        }
    };
    unsafe {
        command.pre_exec(follow_surt);
    }
}

#[cfg(not(target_os = "linux"))]
fn end_with_surt(_command: &mut tokio::process::Command) {}
