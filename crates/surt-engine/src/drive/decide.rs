mod plan;
mod rules;

use super::graph::{BlockReason, OperationId, RunGraph, RunId, RunState, StepRunId};
use plan::Plan;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    StartIteration {
        loop_: StepRunId,
    },
    ExpandItems {
        step: StepRunId,
    },
    Launch {
        step: StepRunId,
    },
    RunOperation {
        step: StepRunId,
        op: OperationId,
    },
    OpenGate {
        step: StepRunId,
    },
    Skip {
        step: StepRunId,
    },
    Block {
        step: StepRunId,
        reason: BlockReason,
    },
    FinishStep {
        step: StepRunId,
    },
    FinishRun {
        run: RunId,
        end: RunEnd,
    },
    Return(DriveOutcome),
    Wait,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunEnd {
    Succeeded,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DriveOutcome {
    AtGate { gates: Vec<StepRunId> },
    Succeeded,
    Blocked { steps: Vec<StepRunId> },
    Cancelled,
    Interrupted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Limits {
    pub(crate) max_attempts: usize,
    pub(crate) max_iterations: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StopLevel {
    None,
    Graceful,
    Force,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ctx {
    pub(crate) limits: Limits,
    pub(crate) free_slots: usize,
    pub(crate) stop: StopLevel,
}

pub(crate) fn decide(graph: &RunGraph, ctx: &Ctx) -> Vec<Action> {
    let root = &graph.root;
    match root.state {
        RunState::Succeeded => vec![Action::Return(DriveOutcome::Succeeded)],
        RunState::Cancelled => vec![Action::Return(DriveOutcome::Cancelled)],
        RunState::Active if graph.cancel_requested => vec![Action::FinishRun {
            run: root.id.clone(),
            end: RunEnd::Cancelled,
        }],
        RunState::Active => Plan::new(ctx).visit(root),
    }
}

#[cfg(test)]
mod tests;
