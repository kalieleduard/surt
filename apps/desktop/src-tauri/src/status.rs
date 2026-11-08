use serde::Serialize;
use surt_engine::project::{
    BlockReason, BlockedStep, GateKind, GateStatus, GateView, ItemState, ItemStatus, Project,
    RunState, RunStatus,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    root: String,
    name: String,
    max_attempts: u32,
    max_iterations: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunsView {
    slots: SlotsView,
    subjects: Vec<SubjectView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SlotsView {
    in_use: u32,
    total: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubjectView {
    name: String,
    run: u32,
    runs: u32,
    states: Vec<&'static str>,
    ended_at: Option<i64>,
    driven: bool,
    branch: Option<String>,
    item_step: Option<String>,
    items: Vec<ItemView>,
    gates: Vec<GateSummary>,
    blocked: Vec<BlockedView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemView {
    key: String,
    state: &'static str,
    step: Option<String>,
    iteration: Option<u32>,
    attempt: Option<u32>,
    since: Option<i64>,
    merged_as: Option<String>,
    needs: Vec<String>,
    waits_for: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GateSummary {
    step: String,
    kind: &'static str,
    artifact: String,
    status: &'static str,
    revision: Option<u32>,
    decided_by: Option<String>,
    decided_at: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BlockedView {
    step: String,
    item: Option<String>,
    iteration: Option<u32>,
    reason: &'static str,
    failure: Option<String>,
}

impl ProjectView {
    pub fn of(project: &Project) -> ProjectView {
        let root = project.root();
        ProjectView {
            root: root.display().to_string(),
            name: root
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
            max_attempts: project.config().max_attempts.get(),
            max_iterations: project.config().max_iterations.get(),
        }
    }
}

impl RunsView {
    pub fn of(project: &Project, statuses: Vec<RunStatus>) -> RunsView {
        let slots = statuses.first().map_or(
            SlotsView {
                in_use: 0,
                total: project.config().max_concurrent_agents.get(),
            },
            |status| SlotsView {
                in_use: status.slots.in_use,
                total: status.slots.total,
            },
        );
        RunsView {
            slots,
            subjects: statuses.into_iter().map(SubjectView::from).collect(),
        }
    }
}

impl From<RunStatus> for SubjectView {
    fn from(status: RunStatus) -> SubjectView {
        SubjectView {
            name: status.subject,
            run: status.run,
            runs: status.runs,
            states: status.states.into_iter().map(run_state).collect(),
            ended_at: status.ended_at,
            driven: status.driver.is_some(),
            branch: status.branch.map(|branch| branch.name),
            item_step: status.item_step,
            items: status.items.into_iter().map(ItemView::from).collect(),
            gates: status.gates.into_iter().map(GateSummary::from).collect(),
            blocked: status.blocked.into_iter().map(BlockedView::from).collect(),
        }
    }
}

impl From<ItemStatus> for ItemView {
    fn from(item: ItemStatus) -> ItemView {
        ItemView {
            key: item.key,
            state: item_state(item.state),
            step: item.step,
            iteration: item.iteration,
            attempt: item.attempt,
            since: item.since,
            merged_as: item.merged_as,
            needs: item.needs,
            waits_for: item.waits_for,
        }
    }
}

impl From<GateView> for GateSummary {
    fn from(gate: GateView) -> GateSummary {
        let (status, revision) = gate_status(gate.status);
        GateSummary {
            step: gate.step,
            kind: match gate.kind {
                GateKind::Approval => "approval",
                GateKind::Input => "input",
            },
            artifact: gate.artifact,
            status,
            revision,
            decided_by: gate.decided_by,
            decided_at: gate.decided_at,
        }
    }
}

impl From<BlockedStep> for BlockedView {
    fn from(blocked: BlockedStep) -> BlockedView {
        BlockedView {
            step: blocked.step.key,
            item: blocked.step.item,
            iteration: blocked.step.iteration,
            reason: match blocked.reason {
                BlockReason::AttemptsExhausted => "attempts_exhausted",
                BlockReason::NotRetryable => "not_retryable",
                BlockReason::IterationsExhausted => "iterations_exhausted",
            },
            failure: blocked.last_failure.map(|failure| failure.message),
        }
    }
}

fn run_state(state: RunState) -> &'static str {
    match state {
        RunState::Running => "running",
        RunState::AwaitingApproval => "awaiting_approval",
        RunState::AwaitingAnswer => "awaiting_answer",
        RunState::Blocked => "blocked",
        RunState::Interrupted => "interrupted",
        RunState::Succeeded => "succeeded",
        RunState::Cancelled => "cancelled",
    }
}

fn item_state(state: ItemState) -> &'static str {
    match state {
        ItemState::Pending => "pending",
        ItemState::Queued => "queued",
        ItemState::Running => "running",
        ItemState::Interrupted => "interrupted",
        ItemState::Blocked => "blocked",
        ItemState::Succeeded => "succeeded",
        ItemState::Cancelled => "cancelled",
    }
}

fn gate_status(status: GateStatus) -> (&'static str, Option<u32>) {
    match status {
        GateStatus::NotReached => ("not_reached", None),
        GateStatus::Waiting { revision } => ("waiting", Some(revision)),
        GateStatus::Approved { revision } => ("approved", Some(revision)),
        GateStatus::Rejected { revision } => ("rejected", Some(revision)),
        GateStatus::Answered { revision } => ("answered", Some(revision)),
    }
}
