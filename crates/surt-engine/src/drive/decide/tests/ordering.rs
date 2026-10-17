use super::super::{Action, decide};
use super::fixtures::{agent, approval, ctx, id, launch, operation, root};
use crate::drive::graph::{ExecutionState, StepRunState};

#[test]
fn a_step_launches_only_once_every_step_it_needs_finished() {
    let table = [
        (agent("a").done("done"), vec![launch("b")]),
        (
            agent("a").in_state(StepRunState::Skipped),
            vec![launch("b")],
        ),
        (
            agent("a").running().executions(&[ExecutionState::Running]),
            vec![Action::Wait],
        ),
    ];

    for (dependency, expected) in table {
        let graph = root(vec![dependency, agent("b").needs(&["a"])]);

        assert_eq!(decide(&graph, &ctx()), expected);
    }
}

#[test]
fn a_step_runs_only_when_its_condition_matches_and_is_skipped_otherwise() {
    let skip = Action::Skip { step: id("review") };
    let table = [
        (
            operation("check", "checks.run").done("passed"),
            launch("review"),
        ),
        (
            operation("check", "checks.run").done("failed"),
            skip.clone(),
        ),
        (
            operation("check", "checks.run").in_state(StepRunState::Skipped),
            skip,
        ),
    ];

    for (check, expected) in table {
        let review = agent("review").needs(&["check"]).when("check", "passed");
        let graph = root(vec![check, review]);

        assert_eq!(decide(&graph, &ctx()), vec![expected]);
    }
}

#[test]
fn an_approval_gate_opens_once_the_steps_it_needs_finished() {
    let graph = root(vec![
        agent("spec").done("done"),
        approval("approve-spec").needs(&["spec"]),
    ]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::OpenGate {
            step: id("approve-spec")
        }]
    );
}
