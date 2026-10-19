use surt_engine::project::{MergeIntoBase, RunStatus};

use crate::continue_run::new_run_command;
use crate::message::Message;
use crate::status::merge_into_base;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MergeFlag {
    Merge,
    NoMerge,
}

impl MergeFlag {
    pub(crate) fn chosen(merge: bool, no_merge: bool) -> Option<MergeFlag> {
        match (merge, no_merge) {
            (true, _) => Some(MergeFlag::Merge),
            (_, true) => Some(MergeFlag::NoMerge),
            _ => None,
        }
    }

    pub(crate) fn mode(self) -> MergeIntoBase {
        match self {
            MergeFlag::Merge => MergeIntoBase::FfOnly,
            MergeFlag::NoMerge => MergeIntoBase::Never,
        }
    }

    fn name(self) -> &'static str {
        match self {
            MergeFlag::Merge => "--merge",
            MergeFlag::NoMerge => "--no-merge",
        }
    }
}

pub(crate) fn with_merge_line(
    message: Message,
    mode: MergeIntoBase,
    flag: Option<MergeFlag>,
) -> Message {
    let line = format!("merge into base: {}", merge_into_base(mode));
    match flag {
        Some(flag) => message.context(format!("{line} ({})", flag.name())),
        None if mode == MergeIntoBase::Never => message,
        None => message.context(line),
    }
}

pub(crate) fn already_frozen(subject: &str, status: &RunStatus, flag: MergeFlag) -> Message {
    Message::new(format!(
        "run {} of {subject} already froze merge_into_base = \"{}\"",
        status.run,
        merge_into_base(status.merge_into_base)
    ))
    .way(
        "to choose again, start a new run",
        format!("{} {}", new_run_command(subject), flag.name()),
    )
}
