use surt_engine::project::{BaseMerge, NotMerged};

use crate::json::Fields;

const MERGED: &str = "merged";
const SKIPPED: &str = "skipped";

pub(super) fn merge(merge: &BaseMerge) -> Fields {
    match merge {
        BaseMerge::Merged { from, to } => Fields::new()
            .with("outcome", MERGED)
            .with("from", from.as_str())
            .with("to", to.as_str()),
        BaseMerge::Skipped(not_merged) => {
            let skipped = Fields::new()
                .with("outcome", SKIPPED)
                .with("reason", reason(not_merged));
            match not_merged {
                NotMerged::BaseMoved { found } => skipped.maybe("found", found.as_deref()),
                _ => skipped,
            }
        }
    }
}

fn reason(not_merged: &NotMerged) -> &'static str {
    match not_merged {
        NotMerged::Disabled => "disabled",
        NotMerged::NoBaseBranch => "no_base_branch",
        NotMerged::BaseMoved { .. } => "base_moved",
        NotMerged::WorktreeNotClean => "worktree_not_clean",
    }
}
