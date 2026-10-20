use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    pub(super) runtime: RuntimeKind,
    pub(crate) model: String,
    pub(crate) effort: Effort,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) limits: Option<Limits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) claude: Option<ClaudeProfile>,
    #[serde(default, skip_serializing_if = "ConversationScope::is_none")]
    pub(crate) conversation: ConversationScope,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConversationScope {
    #[default]
    None,
    Loop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(super) enum RuntimeKind {
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "fake")]
    Fake,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Limits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) max_turns: Option<NonZeroU32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) max_budget_usd: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClaudeProfile {
    pub(crate) auth: Auth,
    pub(crate) permission_mode: PermissionMode,
    pub(crate) tools: Vec<String>,
    pub(crate) allowed_tools: Vec<String>,
    pub(crate) disallowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) api_key_env: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Auth {
    Subscription,
    ApiKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum PermissionMode {
    #[serde(rename = "dontAsk")]
    DontAsk,
    #[serde(rename = "bypassPermissions")]
    BypassPermissions,
}

impl ConversationScope {
    fn is_none(&self) -> bool {
        *self == ConversationScope::None
    }
}
