use super::rules::{
    NextAttempt, all_finished, finished, holds, iteration_allowed, next_attempt, succeeded,
};
use super::{Action, Ctx, Decision, DriveOutcome, RunEnd, StopLevel};
use crate::drive::graph::{
    Attempts, BlockReason, Condition, Item, Iterations, RunState, Step, StepRunId, StepRunState,
    Work, WorkflowRun,
};

pub(super) struct Plan<'a> {
    ctx: &'a Ctx,
    free_slots: std::slice::Iter<'a, usize>,
    actions: Vec<Action>,
    live: bool,
    gates: Vec<StepRunId>,
    blocked: Vec<StepRunId>,
    queued: usize,
}

impl<'a> Plan<'a> {
    pub(super) fn new(ctx: &'a Ctx) -> Self {
        Plan {
            ctx,
            free_slots: ctx.free_slots.iter(),
            actions: Vec::new(),
            live: false,
            gates: Vec::new(),
            blocked: Vec::new(),
            queued: 0,
        }
    }

    pub(super) fn visit(mut self, root: &WorkflowRun) -> Decision {
        self.visit_run(root);
        let queued = self.queued;
        Decision {
            actions: self.finish(),
            queued,
        }
    }

    fn visit_run(&mut self, run: &WorkflowRun) {
        if all_finished(&run.steps) {
            self.actions.push(Action::FinishRun {
                run: run.id.clone(),
                end: RunEnd::Succeeded,
            });
        } else {
            self.visit_steps(&run.steps);
        }
    }

    fn visit_steps(&mut self, scope: &[Step]) {
        for step in scope {
            match &step.state {
                StepRunState::Pending => self.start(step, scope),
                StepRunState::Running => self.advance(step),
                StepRunState::Blocked(_) => self.blocked.push(step.id.clone()),
                StepRunState::Done(_) | StepRunState::Skipped | StepRunState::Cancelled => {}
            }
        }
    }

    fn start(&mut self, step: &Step, scope: &[Step]) {
        let mut awaited = step
            .needs
            .iter()
            .chain(step.when.iter().map(|when| &when.step));
        if !awaited.all(|key| finished(scope, key)) {
            return;
        }
        match &step.when {
            Some(when) if !holds(scope, when) => self.actions.push(Action::Skip {
                step: step.id.clone(),
            }),
            _ => self.work_on(step),
        }
    }

    fn advance(&mut self, step: &Step) {
        match &step.work {
            Work::Agent(attempts) | Work::Operation(_, attempts) => self.retry(step, attempts),
            Work::Approval => self.gates.push(step.id.clone()),
            Work::Loop(until, iterations) => self.iterate(step, until, iterations),
            Work::PerItem(items) => self.visit_items(step, items),
        }
    }

    fn retry(&mut self, step: &Step, attempts: &Attempts) {
        match next_attempt(attempts, self.ctx.limits.max_attempts) {
            NextAttempt::InProgress => self.live = true,
            NextAttempt::Retry => self.work_on(step),
            NextAttempt::Exhausted(reason) => self.block(step, reason),
        }
    }

    fn iterate(&mut self, step: &Step, until: &Condition, iterations: &Iterations) {
        let current = iterations
            .bodies
            .last()
            .map(Vec::as_slice)
            .unwrap_or_default();
        if !all_finished(current) {
            self.visit_steps(current);
        } else if holds(current, until) {
            self.finish_step(step);
        } else if iteration_allowed(iterations, self.ctx.limits.max_iterations) {
            self.work_on(step);
        } else {
            self.block(step, BlockReason::IterationsExhausted);
        }
    }

    fn visit_items(&mut self, step: &Step, items: &[Item]) {
        if items
            .iter()
            .all(|item| item.run.state == RunState::Succeeded)
        {
            self.finish_step(step);
            return;
        }
        for item in items {
            let startable = item.needs.iter().all(|need| succeeded(items, need));
            if item.run.state == RunState::Active && startable {
                self.visit_run(&item.run);
            }
        }
    }

    fn work_on(&mut self, step: &Step) {
        let id = step.id.clone();
        let action = match &step.work {
            Work::Agent(_) => {
                let Some(&slot) = self.free_slots.next() else {
                    self.queued += 1;
                    return;
                };
                Action::Launch { step: id, slot }
            }
            Work::Operation(op, _) => Action::RunOperation {
                step: id,
                op: op.clone(),
            },
            Work::Approval => Action::OpenGate { step: id },
            Work::Loop(..) => Action::StartIteration { loop_: id },
            Work::PerItem(_) => Action::ExpandItems { step: id },
        };
        self.actions.push(action);
    }

    fn finish_step(&mut self, step: &Step) {
        self.actions.push(Action::FinishStep {
            step: step.id.clone(),
        });
    }

    fn block(&mut self, step: &Step, reason: BlockReason) {
        self.actions.push(Action::Block {
            step: step.id.clone(),
            reason,
        });
    }

    fn finish(self) -> Vec<Action> {
        let stopping = self.ctx.stop != StopLevel::None;
        let waiting_for_slot = self.queued > 0 && !stopping;
        let outcome = if stopping {
            DriveOutcome::Interrupted
        } else if !self.actions.is_empty() {
            return self.actions;
        } else if !self.blocked.is_empty() {
            DriveOutcome::Blocked {
                steps: self.blocked,
            }
        } else {
            DriveOutcome::AtGate { gates: self.gates }
        };
        if self.live || waiting_for_slot {
            return vec![Action::Wait];
        }
        vec![Action::Return(outcome)]
    }
}
