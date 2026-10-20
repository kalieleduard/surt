use crate::drive::graph::{
    Attempts, BlockReason, Condition, ExecutionState, FailureClass, Item, ItemKey, Iterations,
    RunState, Step, StepKey, StepRunState, Work,
};

pub(super) enum NextAttempt {
    InProgress,
    Retry,
    Exhausted(BlockReason),
}

pub(super) fn next_attempt(attempts: &Attempts, max_attempts: usize) -> NextAttempt {
    let budget = attempts
        .executions
        .get(attempts.budget_start..)
        .unwrap_or_default();
    let spent = budget
        .iter()
        .filter(|execution| spends_attempt(**execution))
        .count();
    match budget.last() {
        Some(ExecutionState::Queued | ExecutionState::Running) => NextAttempt::InProgress,
        Some(ExecutionState::Failed(FailureClass::NotRetryable)) => {
            NextAttempt::Exhausted(BlockReason::NotRetryable)
        }
        _ if spent >= max_attempts => NextAttempt::Exhausted(BlockReason::AttemptsExhausted),
        _ => NextAttempt::Retry,
    }
}

fn spends_attempt(execution: ExecutionState) -> bool {
    matches!(
        execution,
        ExecutionState::Failed(FailureClass::Retryable | FailureClass::NotRetryable)
    )
}

pub(super) fn iteration_allowed(iterations: &Iterations, max_iterations: usize) -> bool {
    let current = iterations
        .bodies
        .last()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let spent = iterations
        .bodies
        .len()
        .saturating_sub(iterations.budget_start);
    gated(current) || spent < max_iterations
}

fn gated(body: &[Step]) -> bool {
    body.iter()
        .any(|step| matches!(step.work, Work::Approval | Work::Input))
}

pub(super) fn holds(scope: &[Step], condition: &Condition) -> bool {
    scope.iter().any(|step| {
        step.key == condition.step
            && matches!(&step.state, StepRunState::Done(outcome) if *outcome == condition.outcome)
    })
}

pub(super) fn all_finished(scope: &[Step]) -> bool {
    scope.iter().all(|step| finished(scope, &step.key))
}

pub(super) fn finished(scope: &[Step], key: &StepKey) -> bool {
    scope.iter().any(|step| {
        &step.key == key && matches!(step.state, StepRunState::Done(_) | StepRunState::Skipped)
    })
}

pub(super) fn succeeded(items: &[Item], key: &ItemKey) -> bool {
    items
        .iter()
        .any(|item| &item.key == key && item.run.state == RunState::Succeeded)
}
