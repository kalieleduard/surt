use surt_engine::project::{BaseMerge, NotMerged};

use crate::json::Fields;

pub(super) fn merge(merge: &BaseMerge) -> Fields {
    let outcome = Fields::new().with("outcome", merge.outcome());
    match merge {
        BaseMerge::Merged {
            from,
            to,
            merge_commit,
        } => {
            let merged = outcome.with("from", from.as_str()).with("to", to.as_str());
            if *merge_commit {
                merged.with("merge_commit", true)
            } else {
                merged
            }
        }
        BaseMerge::Skipped(not_merged) => {
            skipped(outcome.with("reason", not_merged.reason()), not_merged)
        }
    }
}

fn skipped(fields: Fields, not_merged: &NotMerged) -> Fields {
    match not_merged {
        NotMerged::BaseMoved { found } => fields.maybe("found", found.as_deref()),
        NotMerged::WorktreeNotClean { worktree } => {
            fields.with("worktree", worktree.display().to_string())
        }
        NotMerged::Conflict { paths } => fields.with("paths", paths.clone()),
        NotMerged::Disabled | NotMerged::NoBaseBranch => fields,
    }
}
