use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum StepSpec {
    Agent(AgentStep),
    Approval(ApprovalStep),
    Input(InputStep),
    Operation(OperationStep),
    Loop(LoopStep),
    PerItem(PerItemStep),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StepKind {
    Agent,
    Approval,
    Input,
    Operation,
    Loop,
    PerItem,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Condition {
    pub(crate) step: String,
    pub(crate) outcome: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentStep {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    needs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<Condition>,
    pub(crate) agent: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) inputs: BTreeMap<String, String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApprovalStep {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    needs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<Condition>,
    pub(crate) artifact: String,
    pub(crate) rejection_artifact: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputStep {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    needs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<Condition>,
    pub(crate) artifact: String,
    pub(crate) answer_artifact: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OperationStep {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    needs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<Condition>,
    pub(crate) run: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LoopStep {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    needs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<Condition>,
    pub(crate) body: Vec<String>,
    pub(crate) until: Condition,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PerItemStep {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    needs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    when: Option<Condition>,
    pub(crate) items: String,
    pub(crate) workflow: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) pass: Vec<String>,
}

impl StepKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            StepKind::Agent => "agent",
            StepKind::Approval => "approval",
            StepKind::Input => "input",
            StepKind::Operation => "operation",
            StepKind::Loop => "loop",
            StepKind::PerItem => "per_item",
        }
    }
}

impl StepSpec {
    pub(crate) fn kind(&self) -> StepKind {
        match self {
            StepSpec::Agent(_) => StepKind::Agent,
            StepSpec::Approval(_) => StepKind::Approval,
            StepSpec::Input(_) => StepKind::Input,
            StepSpec::Operation(_) => StepKind::Operation,
            StepSpec::Loop(_) => StepKind::Loop,
            StepSpec::PerItem(_) => StepKind::PerItem,
        }
    }

    pub(crate) fn needs(&self) -> &[String] {
        match self {
            StepSpec::Agent(step) => &step.needs,
            StepSpec::Approval(step) => &step.needs,
            StepSpec::Input(step) => &step.needs,
            StepSpec::Operation(step) => &step.needs,
            StepSpec::Loop(step) => &step.needs,
            StepSpec::PerItem(step) => &step.needs,
        }
    }

    pub(crate) fn when(&self) -> Option<&Condition> {
        match self {
            StepSpec::Agent(step) => step.when.as_ref(),
            StepSpec::Approval(step) => step.when.as_ref(),
            StepSpec::Input(step) => step.when.as_ref(),
            StepSpec::Operation(step) => step.when.as_ref(),
            StepSpec::Loop(step) => step.when.as_ref(),
            StepSpec::PerItem(step) => step.when.as_ref(),
        }
    }
}
