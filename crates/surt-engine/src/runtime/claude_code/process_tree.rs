#[cfg(unix)]
use process_wrap::tokio::ProcessGroup;
use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
#[cfg(windows)]
use process_wrap::tokio::{CreationFlags, JobObject};

#[cfg(unix)]
const SIGTERM: i32 = 15;

pub(super) fn contain(command: &mut CommandWrap) {
    #[cfg(unix)]
    command.wrap(ProcessGroup::leader());
    #[cfg(target_os = "linux")]
    end_with_surt(command.command_mut());
    #[cfg(windows)]
    command
        .wrap(CreationFlags(
            windows::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP,
        ))
        .wrap(JobObject);
    command.wrap(KillOnDrop);
}

#[cfg(unix)]
pub(super) fn interrupt(child: &mut dyn ChildWrapper) {
    let _ = child.signal(SIGTERM);
}

#[cfg(windows)]
pub(super) fn interrupt(child: &mut dyn ChildWrapper) {
    use windows::Win32::System::Console::{CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent};

    let delivered = child
        .id()
        .is_some_and(|group| unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, group) }.is_ok());
    if !delivered {
        let _ = child.start_kill();
    }
}

#[cfg(target_os = "linux")]
fn end_with_surt(command: &mut tokio::process::Command) {
    use nix::errno::Errno;
    use nix::sys::prctl::set_pdeathsig;
    use nix::sys::signal::Signal;
    use nix::unistd::{getpid, getppid};

    let surt = getpid();
    let follow_surt = move || -> std::io::Result<()> {
        set_pdeathsig(Signal::SIGTERM)?;
        if getppid() == surt {
            Ok(())
        } else {
            Err(Errno::ESRCH.into())
        }
    };
    unsafe {
        command.pre_exec(follow_surt);
    }
}
