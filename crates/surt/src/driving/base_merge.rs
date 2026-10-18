use surt_engine::project::{BaseMerge, BaseMergeReport, NotMerged};

use super::short;
use crate::message::{Mark, Message};
use crate::symbols::ARROW;

const UNNAMED_BRANCH: &str = "the base branch";

pub(crate) fn line(report: &BaseMergeReport, integration: &str) -> Option<(Mark, Message)> {
    let branch = report.base.branch.as_deref().unwrap_or(UNNAMED_BRANCH);
    let base = short(&report.base.sha);
    let why = match &report.merge {
        BaseMerge::Merged { from, to } => {
            let text = format!(
                "merged into {branch} (fast-forward {} {ARROW} {})",
                short(from),
                short(to)
            );
            return Some((Mark::Success, Message::new(text)));
        }
        BaseMerge::Skipped(NotMerged::Disabled) => return None,
        BaseMerge::Skipped(NotMerged::NoBaseBranch) => {
            format!("the base {base} is not a branch")
        }
        BaseMerge::Skipped(NotMerged::BaseMoved { found: Some(found) }) => {
            format!(
                "{branch} moved since base ({base} {ARROW} {})",
                short(found)
            )
        }
        BaseMerge::Skipped(NotMerged::BaseMoved { found: None }) => {
            format!("{branch} no longer exists (base {base})")
        }
        BaseMerge::Skipped(NotMerged::WorktreeNotClean) => {
            format!("{branch} is checked out in a worktree that is not clean")
        }
    };
    let text = format!("not merged: {why}; merge {integration} yourself");
    Some((Mark::Warning, Message::new(text)))
}

#[cfg(test)]
mod tests;
