use surt_engine::project::{Cleanup, Kept, Left};

use crate::message::{Mark, Message};
use crate::symbols::{DASH, SEPARATOR};

const NOTHING_LEFT: &str = "done";

pub(crate) fn report(cleanup: &Cleanup, retry: String) {
    if cleanup.is_complete() {
        Message::new(summary(cleanup)).print(Mark::Retry);
    }
    for kept in &cleanup.kept {
        Message::new(kept_line(kept)).print(Mark::Warning);
    }
    if let Some((first, rest)) = cleanup.left.split_first() {
        let mut message = Message::new(format!("cleanup incomplete {DASH} {}", left_line(first)));
        for left in rest {
            message = message.context(left_line(left));
        }
        message.way("retry", retry).print(Mark::Warning);
    }
}

pub(super) fn summary(cleanup: &Cleanup) -> String {
    let mut parts = Vec::new();
    for (count, one, many) in [
        (cleanup.worktrees, "worktree", "worktrees"),
        (cleanup.child_branches, "task branch", "task branches"),
        (cleanup.salvage_refs, "salvage ref", "salvage refs"),
    ] {
        match count {
            0 => {}
            1 => parts.push(format!("1 {one}")),
            _ => parts.push(format!("{count} {many}")),
        }
    }
    if cleanup.working_copy {
        parts.push("working copy".to_owned());
    }
    if parts.is_empty() {
        return format!("cleanup {NOTHING_LEFT}");
    }
    format!("cleanup {}", parts.join(SEPARATOR))
}

pub(crate) fn kept_line(kept: &Kept) -> String {
    match kept {
        Kept::UnrecordedEdit { path } => format!("kept {} {DASH} unrecorded edit", path.display()),
        Kept::MovedBranch { branch } => format!("kept {branch} {DASH} moved outside surt"),
        Kept::ForeignPath { path } => {
            format!("kept {} {DASH} not a worktree surt created", path.display())
        }
    }
}

fn left_line(left: &Left) -> String {
    format!("could not remove {} ({})", left.item, left.reason)
}
