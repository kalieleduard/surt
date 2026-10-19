use serde::Deserialize;

use crate::workflow::OnSuccess;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CleanupConfig {
    pub on_success: Option<OnSuccess>,
}

impl CleanupConfig {
    pub(crate) fn effective(self, root_definition: OnSuccess) -> OnSuccess {
        self.on_success.unwrap_or(root_definition)
    }
}
