use std::process::ExitStatus;

use process_wrap::tokio::{ChildWrapper, CommandWrap, CreationFlags, JobObject};
use windows::Win32::System::Threading::PROCESS_CREATION_FLAGS;
use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent};
use windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP;

const UNKNOWN_EXIT_CODE: i32 = -1;

pub(super) fn own_tree(command: &mut CommandWrap) {
    command
        .wrap(CreationFlags(PROCESS_CREATION_FLAGS(
            CREATE_NEW_PROCESS_GROUP,
        )))
        .wrap(JobObject);
}

pub(crate) fn interrupt(child: &mut dyn ChildWrapper) {
    let delivered = child
        .id()
        .is_some_and(|group| unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, group) } != 0);
    if !delivered {
        let _ = child.start_kill();
    }
}

pub(crate) fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(UNKNOWN_EXIT_CODE)
}
