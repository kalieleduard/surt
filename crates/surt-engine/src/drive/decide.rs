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
        slot: usize,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Ctx {
    pub(crate) limits: Limits,
    pub(crate) free_slots: Vec<usize>,
    pub(crate) stop: StopLevel,
    pub(crate) paused_until: Option<i64>,
    pub(crate) now: i64,
}

impl Ctx {
    pub(super) fn paused(&self) -> bool {
        self.paused_until.is_some_and(|until| self.now < until)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Decision {
    pub(crate) actions: Vec<Action>,
    pub(crate) queued: usize,
}

#[cfg(test)]
pub(crate) fn decide(graph: &RunGraph, ctx: &Ctx) -> Vec<Action> {
    decision(graph, ctx).actions
}

pub(crate) fn decision(graph: &RunGraph, ctx: &Ctx) -> Decision {
    let root = &graph.root;
    let only = |action| Decision {
        actions: vec![action],
        queued: 0,
    };
    match root.state {
        RunState::Succeeded => only(Action::Return(DriveOutcome::Succeeded)),
        RunState::Cancelled => only(Action::Return(DriveOutcome::Cancelled)),
        RunState::Active if graph.cancel_requested => only(Action::FinishRun {
            run: root.id.clone(),
            end: RunEnd::Cancelled,
        }),
        RunState::Active => Plan::new(ctx).visit(root),
    }
}

#[cfg(test)]
mod tests;
