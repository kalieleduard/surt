use super::super::{Action, Ctx, DriveOutcome, RunEnd, StopLevel, decide};
use super::fixtures::{agent, approval, ctx, id, launch, root};
use crate::drive::graph::{BlockReason, ExecutionState, RunId, RunState, Step, StepRunState};

fn blocked(key: &str) -> Step {
    agent(key).in_state(StepRunState::Blocked(BlockReason::AttemptsExhausted))
}

#[test]
fn a_run_whose_steps_all_finished_succeeds() {
    let graph = root(vec![
        agent("spec").done("done"),
        agent("review").in_state(StepRunState::Skipped),
    ]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::FinishRun {
            run: RunId("root".into()),
            end: RunEnd::Succeeded,
        }]
    );
}

#[test]
fn a_cancel_request_cancels_the_run_even_while_work_is_running() {
    let mut graph = root(vec![
        agent("build")
            .running()
            .executions(&[ExecutionState::Running]),
        agent("other"),
    ]);
    graph.cancel_requested = true;

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::FinishRun {
            run: RunId("root".into()),
            end: RunEnd::Cancelled,
        }]
    );
}

#[test]
fn a_finished_run_returns_its_ending() {
    let table = [
        (
            RunState::Succeeded,
            agent("spec").done("done"),
            DriveOutcome::Succeeded,
        ),
        (
            RunState::Cancelled,
            agent("spec").in_state(StepRunState::Cancelled),
            DriveOutcome::Cancelled,
        ),
    ];

    for (state, step, outcome) in table {
        let mut graph = root(vec![step]);
        graph.root.state = state;

        assert_eq!(decide(&graph, &ctx()), vec![Action::Return(outcome)]);
    }
}

#[test]
fn exhausted_limits_return_blocked_and_never_end_the_run() {
    let graph = root(vec![blocked("build"), agent("ship").needs(&["build"])]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::Return(DriveOutcome::Blocked {
            steps: vec![id("build")]
        })]
    );
}

#[test]
fn a_run_waiting_only_on_an_approval_gate_returns_at_the_gate() {
    let graph = root(vec![
        agent("spec").done("done"),
        approval("approve-spec").needs(&["spec"]).running(),
        agent("plan").needs(&["approve-spec"]),
    ]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::Return(DriveOutcome::AtGate {
            gates: vec![id("approve-spec")]
        })]
    );
}

#[test]
fn a_blocked_step_outranks_an_open_gate_when_returning() {
    let graph = root(vec![blocked("build"), approval("approve").running()]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::Return(DriveOutcome::Blocked {
            steps: vec![id("build")]
        })]
    );
}

#[test]
fn work_that_runs_without_a_human_goes_on_beside_a_gate_or_a_blocked_step() {
    let graph = root(vec![
        blocked("build"),
        approval("approve").running(),
        agent("docs"),
    ]);

    assert_eq!(decide(&graph, &ctx()), vec![launch("docs")]);
}

#[test]
fn a_stop_request_launches_nothing_and_waits_for_running_work() {
    let table = [
        (
            vec![
                agent("build")
                    .running()
                    .executions(&[ExecutionState::Running]),
                agent("docs"),
            ],
            Action::Wait,
        ),
        (
            vec![agent("docs")],
            Action::Return(DriveOutcome::Interrupted),
        ),
    ];

    for stop in [StopLevel::Graceful, StopLevel::Force] {
        for (steps, expected) in table.clone() {
            let ctx = Ctx { stop, ..ctx() };

            assert_eq!(decide(&root(steps), &ctx), vec![expected]);
        }
    }
}
