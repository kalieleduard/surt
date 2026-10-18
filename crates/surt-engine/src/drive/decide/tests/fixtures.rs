use super::super::{Action, Ctx, Limits, StopLevel};
use crate::drive::graph::{
    Attempts, Condition, ExecutionState, Item, ItemKey, Iterations, OperationId, Outcome, RunGraph,
    RunId, RunState, Step, StepKey, StepRunId, StepRunState, Work, WorkflowRun,
};

pub(super) const LIMITS: Limits = Limits {
    max_attempts: 3,
    max_iterations: 5,
};

pub(super) const NOW: i64 = 1_800_000_000;

pub(super) fn ctx() -> Ctx {
    Ctx {
        limits: LIMITS,
        free_slots: vec![0, 1, 2, 3],
        stop: StopLevel::None,
        paused_until: None,
        now: NOW,
    }
}

pub(super) fn root(steps: Vec<Step>) -> RunGraph {
    RunGraph {
        root: run("root", steps),
        cancel_requested: false,
    }
}

pub(super) fn run(id: &str, steps: Vec<Step>) -> WorkflowRun {
    WorkflowRun {
        id: RunId(id.into()),
        state: RunState::Active,
        steps,
    }
}

pub(super) fn item(key: &str, needs: &[&str], steps: Vec<Step>) -> Item {
    Item {
        key: ItemKey(key.into()),
        needs: needs.iter().map(|need| ItemKey((*need).into())).collect(),
        run: run(key, prefixed(key, steps)),
    }
}

fn prefixed(prefix: &str, steps: Vec<Step>) -> Vec<Step> {
    steps
        .into_iter()
        .map(|mut step| {
            step.id = StepRunId(format!("{prefix}/{}", step.id.0));
            step
        })
        .collect()
}

fn step(key: &str, work: Work) -> Step {
    Step {
        id: StepRunId(key.into()),
        key: StepKey(key.into()),
        needs: Vec::new(),
        when: None,
        state: StepRunState::Pending,
        work,
    }
}

pub(super) fn agent(key: &str) -> Step {
    step(key, Work::Agent(Attempts::default()))
}

pub(super) fn operation(key: &str, op: &str) -> Step {
    step(
        key,
        Work::Operation(OperationId(op.into()), Attempts::default()),
    )
}

pub(super) fn approval(key: &str) -> Step {
    step(key, Work::Approval)
}

pub(super) fn looping(key: &str, until: (&str, &str), bodies: Vec<Vec<Step>>) -> Step {
    let until = Condition {
        step: StepKey(until.0.into()),
        outcome: Outcome(until.1.into()),
    };
    let iterations = Iterations {
        bodies: bodies
            .into_iter()
            .enumerate()
            .map(|(index, body)| prefixed(&format!("{key}#{}", index + 1), body))
            .collect(),
        budget_start: 0,
    };
    step(key, Work::Loop(until, iterations))
}

pub(super) fn per_item(key: &str, items: Vec<Item>) -> Step {
    step(key, Work::PerItem(items))
}

impl Step {
    pub(super) fn needs(mut self, keys: &[&str]) -> Self {
        self.needs = keys.iter().map(|key| StepKey((*key).into())).collect();
        self
    }

    pub(super) fn when(mut self, step: &str, outcome: &str) -> Self {
        self.when = Some(Condition {
            step: StepKey(step.into()),
            outcome: Outcome(outcome.into()),
        });
        self
    }

    pub(super) fn in_state(mut self, state: StepRunState) -> Self {
        self.state = state;
        self
    }

    pub(super) fn running(self) -> Self {
        self.in_state(StepRunState::Running)
    }

    pub(super) fn done(self, outcome: &str) -> Self {
        self.in_state(StepRunState::Done(Outcome(outcome.into())))
    }

    pub(super) fn executions(mut self, history: &[ExecutionState]) -> Self {
        if let Work::Agent(attempts) | Work::Operation(_, attempts) = &mut self.work {
            attempts.executions = history.to_vec();
        }
        self
    }

    pub(super) fn budget_start(mut self, start: usize) -> Self {
        match &mut self.work {
            Work::Agent(attempts) | Work::Operation(_, attempts) => attempts.budget_start = start,
            Work::Loop(_, iterations) => iterations.budget_start = start,
            Work::Approval | Work::PerItem(_) => {}
        }
        self
    }
}

pub(super) fn id(value: &str) -> StepRunId {
    StepRunId(value.into())
}

pub(super) fn launch(step: &str) -> Action {
    launch_in(step, 0)
}

pub(super) fn launch_in(step: &str, slot: usize) -> Action {
    Action::Launch {
        step: id(step),
        slot,
    }
}
