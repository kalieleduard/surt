#[cfg_attr(unix, path = "children/unix.rs")]
#[cfg_attr(windows, path = "children/windows.rs")]
mod native;

use process_wrap::tokio::{CommandWrap, KillOnDrop};

pub(crate) use native::{exit_code, interrupt};

pub(crate) fn contain(command: &mut CommandWrap) {
    native::own_tree(command);
    command.wrap(KillOnDrop);
}
