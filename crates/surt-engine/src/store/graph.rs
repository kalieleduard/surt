use sqlx::FromRow;

use super::revisions::number;
use super::{Store, StoreError};
use crate::drive::graph::{
    BlockReason, ExecutionState, FailureClass, Outcome, RunState, StepRunState,
};

#[derive(Clone, Debug)]
pub(crate) struct RunRows {
    pub(crate) id: String,
    pub(crate) subject: String,
    pub(crate) definition: String,
    pub(crate) state: RunState,
    pub(crate) cancel_requested: bool,
    pub(crate) children: Vec<ChildRow>,
    pub(crate) steps: Vec<StepRow>,
    pub(crate) executions: Vec<ExecutionRow>,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct ChildRow {
    pub(crate) id: String,
    pub(crate) parent_step: String,
    pub(crate) item: String,
    pub(crate) definition: String,
    #[sqlx(try_from = "String")]
    pub(crate) state: RunState,
}

#[derive(Clone, Debug)]
pub(crate) struct StepRow {
    pub(crate) id: String,
    pub(crate) run: String,
    pub(crate) item: Option<String>,
    pub(crate) key: String,
    pub(crate) loop_step: Option<String>,
    pub(crate) iteration: Option<u32>,
    pub(crate) state: StepRunState,
    pub(crate) budget_base: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct ExecutionRow {
    pub(crate) step: String,
    pub(crate) state: ExecutionState,
}

#[derive(FromRow)]
struct RootRow {
    id: String,
    subject: String,
    definition_sha256: String,
    state: String,
    cancel_requested: bool,
}

#[derive(FromRow)]
struct RawStep {
    id: String,
    workflow_run_id: String,
    item_key: Option<String>,
    step_key: String,
    loop_step_run_id: Option<String>,
    iteration: Option<i64>,
    state: String,
    outcome: Option<String>,
    blocked_reason: Option<String>,
    budget_base: i64,
}

#[derive(FromRow)]
struct RawExecution {
    step_run_id: String,
    state: String,
    failure_class: Option<String>,
}

impl Store {
    pub(crate) async fn run_rows(&self, root: &str) -> Result<RunRows, StoreError> {
        let mut snapshot = self.pool.begin().await?;
        let run: RootRow = sqlx::query_as(
            "SELECT w.id, s.name AS subject, w.definition_sha256, w.state, \
             w.cancel_requested_at IS NOT NULL AS cancel_requested \
             FROM workflow_run w JOIN subject s ON s.id = w.subject_id WHERE w.id = ?",
        )
        .bind(root)
        .fetch_one(&mut *snapshot)
        .await?;
        let steps: Vec<RawStep> = sqlx::query_as(
            "SELECT s.id, s.workflow_run_id, w.item_key, s.step_key, s.loop_step_run_id, s.iteration, \
             s.state, s.outcome, s.blocked_reason, s.budget_base FROM step_run s JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.root_run_id = ? ORDER BY s.created_at, s.id",
        )
        .bind(root)
        .fetch_all(&mut *snapshot)
        .await?;
        let children: Vec<ChildRow> = sqlx::query_as(
            "SELECT id, parent_step_run_id AS parent_step, item_key AS item, \
             definition_sha256 AS definition, state FROM workflow_run \
             WHERE root_run_id = ? AND parent_step_run_id IS NOT NULL ORDER BY created_at, id",
        )
        .bind(root)
        .fetch_all(&mut *snapshot)
        .await?;
        let executions: Vec<RawExecution> = sqlx::query_as(
            "SELECT e.step_run_id, e.state, e.failure_class FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.root_run_id = ? ORDER BY e.step_run_id, e.attempt",
        )
        .bind(root)
        .fetch_all(&mut *snapshot)
        .await?;
        snapshot.commit().await?;
        Ok(RunRows {
            id: run.id,
            subject: run.subject,
            definition: run.definition_sha256,
            state: run_state(&run.state),
            cancel_requested: run.cancel_requested,
            children,
            steps: steps.into_iter().map(StepRow::from).collect(),
            executions: executions.into_iter().map(ExecutionRow::from).collect(),
        })
    }
}

impl RunRows {
    pub(crate) fn definition_of(&self, run: &str) -> &str {
        self.children
            .iter()
            .find(|child| child.id == run)
            .map_or(self.definition.as_str(), |child| child.definition.as_str())
    }
}

impl From<RawStep> for StepRow {
    fn from(raw: RawStep) -> StepRow {
        let state = match (raw.state.as_str(), raw.outcome, raw.blocked_reason) {
            ("running", ..) => StepRunState::Running,
            ("done", Some(outcome), _) => StepRunState::Done(Outcome(outcome)),
            ("skipped", ..) => StepRunState::Skipped,
            ("blocked", _, Some(reason)) => StepRunState::Blocked(block_reason(&reason)),
            ("cancelled", ..) => StepRunState::Cancelled,
            _ => StepRunState::Pending,
        };
        StepRow {
            id: raw.id,
            run: raw.workflow_run_id,
            item: raw.item_key,
            key: raw.step_key,
            loop_step: raw.loop_step_run_id,
            iteration: raw.iteration.map(number),
            state,
            budget_base: usize::try_from(raw.budget_base).unwrap_or_default(),
        }
    }
}

impl From<RawExecution> for ExecutionRow {
    fn from(raw: RawExecution) -> ExecutionRow {
        let state = match (raw.state.as_str(), raw.failure_class.as_deref()) {
            ("queued", _) => ExecutionState::Queued,
            ("running", _) => ExecutionState::Running,
            ("succeeded", _) => ExecutionState::Succeeded,
            ("failed", Some("transient")) => ExecutionState::Failed(FailureClass::Transient),
            ("failed", Some("not_retryable")) => ExecutionState::Failed(FailureClass::NotRetryable),
            ("failed", _) => ExecutionState::Failed(FailureClass::Retryable),
            ("cancelled", _) => ExecutionState::Cancelled,
            _ => ExecutionState::Interrupted,
        };
        ExecutionRow {
            step: raw.step_run_id,
            state,
        }
    }
}

fn run_state(state: &str) -> RunState {
    match state {
        "succeeded" => RunState::Succeeded,
        "cancelled" => RunState::Cancelled,
        _ => RunState::Active,
    }
}

impl From<String> for RunState {
    fn from(state: String) -> RunState {
        run_state(&state)
    }
}

fn block_reason(reason: &str) -> BlockReason {
    match reason {
        "not_retryable" => BlockReason::NotRetryable,
        "iterations_exhausted" => BlockReason::IterationsExhausted,
        _ => BlockReason::AttemptsExhausted,
    }
}

pub(super) fn block_reason_name(reason: BlockReason) -> &'static str {
    match reason {
        BlockReason::AttemptsExhausted => "attempts_exhausted",
        BlockReason::NotRetryable => "not_retryable",
        BlockReason::IterationsExhausted => "iterations_exhausted",
    }
}

pub(super) fn failure_class_name(class: FailureClass) -> &'static str {
    match class {
        FailureClass::Retryable => "retryable",
        FailureClass::NotRetryable => "not_retryable",
        FailureClass::Transient => "transient",
    }
}
