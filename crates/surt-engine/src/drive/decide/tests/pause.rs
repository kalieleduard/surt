use super::super::{Action, Ctx, DriveOutcome, StopLevel, decide, decision};
use super::fixtures::{NOW, agent, ctx, id, launch, launch_in, operation, root};
use crate::drive::graph::{ExecutionState, FailureClass, OperationId};

const TRANSIENT: ExecutionState = ExecutionState::Failed(FailureClass::Transient);

fn paused_until(until: i64) -> Ctx {
    Ctx {
        paused_until: Some(until),
        ..ctx()
    }
}

fn run_check() -> Action {
    Action::RunOperation {
        step: id("check"),
        op: OperationId("checks.run".into()),
    }
}

#[test]
fn while_paused_no_agent_launches_but_operations_still_run() {
    let graph = root(vec![
        agent("build").running().executions(&[TRANSIENT]),
        agent("review"),
        operation("check", "checks.run"),
    ]);

    let decided = decision(&graph, &paused_until(NOW + 60));

    assert_eq!(decided.actions, vec![run_check()]);
    assert_eq!(decided.queued, 2, "both agents wait for the pause to end");
}

#[test]
fn a_paused_driver_with_nothing_else_to_do_waits_instead_of_returning() {
    let table: [(&str, Vec<_>); 2] = [
        (
            "the agent that failed transient",
            vec![agent("build").running().executions(&[TRANSIENT])],
        ),
        ("a first launch", vec![agent("spec")]),
    ];

    for (case, steps) in table {
        let graph = root(steps);

        assert_eq!(
            decide(&graph, &paused_until(NOW + 60)),
            vec![Action::Wait],
            "{case}"
        );
    }
}

#[test]
fn once_the_pause_ends_launches_resume_up_to_the_free_slots() {
    let table = [("the end of the pause", NOW), ("an older pause", NOW - 600)];

    for (case, until) in table {
        let graph = root(vec![
            agent("build").running().executions(&[TRANSIENT]),
            agent("review"),
        ]);
        let ctx = Ctx {
            free_slots: vec![1, 2],
            ..paused_until(until)
        };

        assert_eq!(
            decide(&graph, &ctx),
            vec![launch_in("build", 1), launch_in("review", 2)],
            "{case}"
        );
    }
}

#[test]
fn an_unpaused_driver_retries_a_transient_failure_at_once() {
    let graph = root(vec![agent("build").running().executions(&[TRANSIENT])]);

    assert_eq!(decide(&graph, &ctx()), vec![launch("build")]);
}

#[test]
fn a_stop_during_the_pause_interrupts_the_drive() {
    let graph = root(vec![agent("build").running().executions(&[TRANSIENT])]);
    let ctx = Ctx {
        stop: StopLevel::Graceful,
        ..paused_until(NOW + 60)
    };

    assert_eq!(
        decide(&graph, &ctx),
        vec![Action::Return(DriveOutcome::Interrupted)]
    );
}
