use serde_json::Value;
use surt_engine::project::{CleanupState, Kept, Left};

use crate::json::Fields;

pub(super) fn cleanup(state: &CleanupState) -> Fields {
    match state {
        CleanupState::Finished { kept } => Fields::new()
            .with("state", "done")
            .list("kept", kept.iter().map(kept_entry).collect()),
        CleanupState::Pending { left, kept } => Fields::new()
            .with("state", "pending")
            .list("left", left.iter().map(left_entry).collect())
            .list("kept", kept.iter().map(kept_entry).collect()),
        CleanupState::Kept => Fields::new().with("state", "kept"),
    }
}

fn left_entry(left: &Left) -> Value {
    Fields::new()
        .with("item", left.item.as_str())
        .with("reason", left.reason.as_str())
        .into()
}

fn kept_entry(kept: &Kept) -> Value {
    let fields = match kept {
        Kept::UnrecordedEdit { path } => Fields::new()
            .with("kind", "unrecorded_edit")
            .with("path", path.to_string_lossy().as_ref()),
        Kept::MovedBranch { branch } => Fields::new()
            .with("kind", "moved_branch")
            .with("branch", branch.as_str()),
        Kept::ForeignPath { path } => Fields::new()
            .with("kind", "foreign_path")
            .with("path", path.to_string_lossy().as_ref()),
    };
    fields.into()
}
