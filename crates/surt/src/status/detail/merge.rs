use surt_engine::project::{BaseMerge, NotMerged, RunStatus};

use crate::driving::short;
use crate::symbols::ARROW;

const UNNAMED_BRANCH: &str = "the base branch";

pub(super) fn result(status: &RunStatus) -> Option<String> {
    let base = status.base.as_ref();
    let branch = base
        .and_then(|base| base.branch.as_deref())
        .unwrap_or(UNNAMED_BRANCH);
    let reason = match status.merge.as_ref()? {
        BaseMerge::Merged { from, to } => {
            return Some(format!(
                "merged into {branch} {} {ARROW} {}",
                short(from),
                short(to)
            ));
        }
        BaseMerge::Skipped(NotMerged::Disabled) => return None,
        BaseMerge::Skipped(NotMerged::NoBaseBranch) => "no base branch".to_owned(),
        BaseMerge::Skipped(NotMerged::WorktreeNotClean) => "worktree not clean".to_owned(),
        BaseMerge::Skipped(NotMerged::BaseMoved { found: None }) => {
            format!("{branch} no longer exists")
        }
        BaseMerge::Skipped(NotMerged::BaseMoved { found: Some(found) }) => match base {
            Some(base) => format!(
                "{branch} moved ({} {ARROW} {})",
                short(&base.sha),
                short(found)
            ),
            None => format!("{branch} moved (now {})", short(found)),
        },
    };
    Some(format!("not merged: {reason}"))
}
