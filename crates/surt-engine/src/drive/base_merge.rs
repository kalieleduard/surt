use serde::{Deserialize, Serialize};

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
    BaseMoved { found: Option<String> },
    WorktreeNotClean,
}

impl BaseMerge {
    pub(crate) fn outcome(&self) -> &'static str {
        match self {
            BaseMerge::Merged { .. } => "merged",
            BaseMerge::Skipped(_) => "skipped",
        }
    }

    pub(crate) fn data(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub(crate) fn from_data(data: &str) -> Option<BaseMerge> {
        serde_json::from_str(data).ok()
    }
}

#[cfg(test)]
mod tests;
