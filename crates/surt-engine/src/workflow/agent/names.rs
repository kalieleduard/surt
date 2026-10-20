use super::{AgentContract, Effort, PermissionMode, RuntimeKind};

impl AgentContract {
    pub(crate) fn runtime(&self) -> &'static str {
        match self.profile.runtime {
            RuntimeKind::ClaudeCode => "claude-code",
            RuntimeKind::Fake => "fake",
        }
    }

    pub(crate) fn effort(&self) -> &'static str {
        self.profile.effort.name()
    }

    pub(crate) fn permission_mode(&self) -> Option<&'static str> {
        self.profile
            .claude
            .as_ref()
            .map(|claude| claude.permission_mode.name())
    }
}

impl Effort {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::Xhigh => "xhigh",
            Effort::Max => "max",
        }
    }
}

impl PermissionMode {
    pub(crate) fn name(self) -> &'static str {
        match self {
            PermissionMode::DontAsk => "dontAsk",
            PermissionMode::BypassPermissions => "bypassPermissions",
        }
    }
}
