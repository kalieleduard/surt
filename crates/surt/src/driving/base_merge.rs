use surt_engine::project::{BaseMerge, BaseMergeReport, Commit, NotMerged};

use super::short;
use crate::message::{Mark, Message};
use crate::symbols::ARROW;

const UNNAMED_BRANCH: &str = "the base branch";
const UNKNOWN_BASE: &str = "unknown";

pub(crate) enum MergeWording {
    Merged(String),
    NotMerged(String),
}

pub(crate) fn line(report: &BaseMergeReport, integration: &str) -> Option<(Mark, Message)> {
    Some(match wording(&report.merge, Some(&report.base))? {
        MergeWording::Merged(text) => (Mark::Success, Message::new(text)),
        MergeWording::NotMerged(why) => {
            let text = format!("not merged: {why}; merge {integration} yourself");
            (Mark::Warning, Message::new(text))
        }
    })
}

pub(crate) fn wording(merge: &BaseMerge, base: Option<&Commit>) -> Option<MergeWording> {
    let branch = base
        .and_then(|base| base.branch.as_deref())
        .unwrap_or(UNNAMED_BRANCH);
    let base = base.map_or(UNKNOWN_BASE, |base| short(&base.sha));
    match merge {
        BaseMerge::Merged { from, to } => Some(MergeWording::Merged(format!(
            "merged into {branch} (fast-forward {} {ARROW} {})",
            short(from),
            short(to)
        ))),
        BaseMerge::Skipped(not_merged) => {
            reason(not_merged, branch, base).map(MergeWording::NotMerged)
        }
    }
}

fn reason(not_merged: &NotMerged, branch: &str, base: &str) -> Option<String> {
    Some(match not_merged {
        NotMerged::Disabled => return None,
        NotMerged::NoBaseBranch => format!("the base {base} is not a branch"),
        NotMerged::BaseMoved { found: Some(found) } => {
            format!(
                "{branch} moved since base ({base} {ARROW} {})",
                short(found)
            )
        }
        NotMerged::BaseMoved { found: None } => format!("{branch} no longer exists (base {base})"),
        NotMerged::WorktreeNotClean { worktree } => format!(
            "{branch} is checked out in {} with changes",
            worktree.display()
        ),
    })
}

#[cfg(test)]
mod tests;
