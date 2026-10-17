use super::super::{Action, decide};
use super::fixtures::{agent, ctx, id, launch, operation, root};
use crate::drive::graph::{BlockReason, ExecutionState, FailureClass, OperationId};

const RETRYABLE: ExecutionState = ExecutionState::Failed(FailureClass::Retryable);
const NOT_RETRYABLE: ExecutionState = ExecutionState::Failed(FailureClass::NotRetryable);
const TRANSIENT: ExecutionState = ExecutionState::Failed(FailureClass::Transient);
const INTERRUPTED: ExecutionState = ExecutionState::Interrupted;

fn block(step: &str, reason: BlockReason) -> Action {
    Action::Block {
        step: id(step),
        reason,
    }
}

#[test]
fn an_agent_step_with_nothing_to_wait_for_launches() {
    let graph = root(vec![agent("spec")]);

    assert_eq!(decide(&graph, &ctx()), vec![launch("spec")]);
}

#[test]
fn failed_attempts_retry_until_max_attempts_then_block() {
    let table: [(&str, &[ExecutionState], Action); 6] = [
        ("no attempt yet", &[], launch("build")),
        ("one failure", &[RETRYABLE], launch("build")),
        ("two failures", &[RETRYABLE, RETRYABLE], launch("build")),
        (
            "max_attempts failures",
            &[RETRYABLE, RETRYABLE, RETRYABLE],
            block("build", BlockReason::AttemptsExhausted),
        ),
        (
            "an attempt is queued",
            &[ExecutionState::Queued],
            Action::Wait,
        ),
        (
            "an attempt is running",
            &[RETRYABLE, ExecutionState::Running],
            Action::Wait,
        ),
    ];

    for (case, history, expected) in table {
        let graph = root(vec![agent("build").running().executions(history)]);

        assert_eq!(decide(&graph, &ctx()), vec![expected], "{case}");
    }
}

#[test]
fn interrupted_and_transient_executions_spend_no_attempt() {
    let table: [(&str, &[ExecutionState], Action); 4] = [
        (
            "five interruptions",
            &[
                INTERRUPTED,
                INTERRUPTED,
                INTERRUPTED,
                INTERRUPTED,
                INTERRUPTED,
            ],
            launch("build"),
        ),
        (
            "two failures and an interruption",
            &[RETRYABLE, RETRYABLE, INTERRUPTED],
            launch("build"),
        ),
        (
            "two failures around a transient one",
            &[RETRYABLE, TRANSIENT, RETRYABLE],
            launch("build"),
        ),
        (
            "a transient failure just happened",
            &[TRANSIENT],
            block("build", BlockReason::NotRetryable),
        ),
    ];

    for (case, history, expected) in table {
        let graph = root(vec![agent("build").running().executions(history)]);

        assert_eq!(decide(&graph, &ctx()), vec![expected], "{case}");
    }
}

#[test]
fn only_failures_that_are_not_transient_count_toward_max_attempts() {
    let table = [
        (ExecutionState::Succeeded, launch("build")),
        (INTERRUPTED, launch("build")),
        (ExecutionState::Cancelled, launch("build")),
        (TRANSIENT, launch("build")),
        (RETRYABLE, block("build", BlockReason::AttemptsExhausted)),
        (
            NOT_RETRYABLE,
            block("build", BlockReason::AttemptsExhausted),
        ),
    ];

    for (earlier, expected) in table {
        let history = [earlier, RETRYABLE, RETRYABLE];
        let graph = root(vec![agent("build").running().executions(&history)]);

        assert_eq!(decide(&graph, &ctx()), vec![expected], "{earlier:?}");
    }
}

#[test]
fn a_not_retryable_failure_blocks_on_its_first_occurrence() {
    let graph = root(vec![agent("build").running().executions(&[NOT_RETRYABLE])]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![block("build", BlockReason::NotRetryable)]
    );
}

#[test]
fn unblocking_renews_the_attempt_budget_and_keeps_the_history() {
    let table: [(&str, &[ExecutionState], usize, Action); 6] = [
        (
            "attempt 4 after three failures",
            &[RETRYABLE, RETRYABLE, RETRYABLE],
            3,
            launch("build"),
        ),
        (
            "attempt 6 after two more failures",
            &[RETRYABLE, RETRYABLE, RETRYABLE, RETRYABLE, RETRYABLE],
            3,
            launch("build"),
        ),
        (
            "renewed budget spent",
            &[RETRYABLE; 6],
            3,
            block("build", BlockReason::AttemptsExhausted),
        ),
        (
            "resumed after a not_retryable failure",
            &[NOT_RETRYABLE],
            1,
            launch("build"),
        ),
        (
            "resumed after a transient failure",
            &[TRANSIENT],
            1,
            launch("build"),
        ),
        (
            "a not_retryable failure after resuming",
            &[NOT_RETRYABLE, NOT_RETRYABLE],
            1,
            block("build", BlockReason::NotRetryable),
        ),
    ];

    for (case, history, budget_start, expected) in table {
        let step = agent("build")
            .running()
            .executions(history)
            .budget_start(budget_start);

        assert_eq!(decide(&root(vec![step]), &ctx()), vec![expected], "{case}");
    }
}

#[test]
fn operation_steps_follow_the_same_attempt_limit() {
    let run_check = Action::RunOperation {
        step: id("check"),
        op: OperationId("checks.run".into()),
    };
    let table: [(&[ExecutionState], Action); 3] = [
        (&[], run_check.clone()),
        (&[RETRYABLE, RETRYABLE], run_check),
        (
            &[RETRYABLE, RETRYABLE, RETRYABLE],
            block("check", BlockReason::AttemptsExhausted),
        ),
    ];

    for (history, expected) in table {
        let step = operation("check", "checks.run")
            .running()
            .executions(history);

        assert_eq!(decide(&root(vec![step]), &ctx()), vec![expected]);
    }
}
