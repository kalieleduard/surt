use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::project::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum BaseMerge {
    Merged { from: String, to: String },
    Skipped(NotMerged),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum NotMerged {
    Disabled,
    NoBaseBranch,
    BaseMoved {
        found: Option<String>,
    },
    WorktreeNotClean {
        #[serde(default)]
        worktree: PathBuf,
    },
}

impl BaseMerge {
    pub fn outcome(&self) -> &'static str {
        match self {
            BaseMerge::Merged { .. } => "merged",
            BaseMerge::Skipped(_) => "skipped",
        }
    }

    pub(crate) fn data(&self) -> Result<Value, Error> {
        serde_json::to_value(self).map_err(Error::internal)
    }

    pub(crate) fn from_data(data: &str) -> Option<BaseMerge> {
        serde_json::from_str(data).ok()
    }
}

impl NotMerged {
    pub fn reason(&self) -> &'static str {
        match self {
            NotMerged::Disabled => "disabled",
            NotMerged::NoBaseBranch => "no_base_branch",
            NotMerged::BaseMoved { .. } => "base_moved",
            NotMerged::WorktreeNotClean { .. } => "worktree_not_clean",
        }
    }
}

#[cfg(test)]
mod tests;
