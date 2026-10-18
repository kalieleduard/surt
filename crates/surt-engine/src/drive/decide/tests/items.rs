use super::super::{Action, Ctx, DriveOutcome, RunEnd, StopLevel, decide};
use super::fixtures::{agent, ctx, id, item, launch, launch_in, operation, per_item, root};
use crate::drive::graph::{
    BlockReason, ExecutionState, Item, OperationId, RunId, RunState, Step, StepRunState,
};

fn task(key: &str, needs: &[&str]) -> Item {
    item(key, needs, vec![agent("build")])
}

fn stuck(key: &str) -> Item {
    let build = agent("build").in_state(StepRunState::Blocked(BlockReason::AttemptsExhausted));
    item(key, &[], vec![build])
}

fn merged(key: &str) -> Item {
    let mut merged = item(key, &[], vec![agent("build").done("done")]);
    merged.run.state = RunState::Succeeded;
    merged
}

fn implement(items: Vec<Item>) -> Step {
    per_item("implement", items).running()
}

#[test]
fn a_per_item_step_whose_needs_finished_expands_its_items() {
    let graph = root(vec![
        agent("plan").done("done"),
        per_item("implement", Vec::new()).needs(&["plan"]),
    ]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::ExpandItems {
            step: id("implement")
        }]
    );
}

#[test]
fn an_item_starts_once_every_item_it_needs_succeeded() {
    let graph = root(vec![implement(vec![merged("A"), task("B", &["A"])])]);

    assert_eq!(decide(&graph, &ctx()), vec![launch("B/build")]);
}

#[test]
fn dependents_of_a_blocked_item_wait_while_independent_items_launch() {
    let graph = root(vec![implement(vec![
        stuck("A"),
        task("B", &["A"]),
        task("C", &[]),
    ])]);

    assert_eq!(decide(&graph, &ctx()), vec![launch("C/build")]);
}

#[test]
fn once_independent_items_merged_a_blocked_item_returns_blocked() {
    let graph = root(vec![implement(vec![
        stuck("A"),
        task("B", &["A"]),
        merged("C"),
    ])]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::Return(DriveOutcome::Blocked {
            steps: vec![id("A/build")]
        })]
    );
}

#[test]
fn a_child_run_whose_steps_all_finished_succeeds() {
    let finished = item("A", &[], vec![agent("build").done("done")]);
    let graph = root(vec![implement(vec![finished])]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::FinishRun {
            run: RunId("A".into()),
            end: RunEnd::Succeeded,
        }]
    );
}

#[test]
fn a_per_item_step_finishes_once_every_child_succeeded() {
    let graph = root(vec![implement(vec![merged("A"), merged("B")])]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::FinishStep {
            step: id("implement")
        }]
    );
}

#[test]
fn independent_items_launch_only_up_to_the_free_agent_slots_each_in_its_own_slot() {
    let table = [
        (
            vec![1, 3],
            vec![launch_in("A/build", 1), launch_in("B/build", 3)],
        ),
        (
            vec![0, 1, 2, 3],
            vec![
                launch_in("A/build", 0),
                launch_in("B/build", 1),
                launch_in("C/build", 2),
            ],
        ),
    ];

    for (free_slots, expected) in table {
        let items = vec![task("A", &[]), task("B", &[]), task("C", &[])];
        let ctx = Ctx {
            free_slots,
            ..ctx()
        };

        assert_eq!(decide(&root(vec![implement(items)]), &ctx), expected);
    }
}

#[test]
fn with_no_free_slot_agents_wait_while_operations_still_run() {
    let busy = agent("build")
        .running()
        .executions(&[ExecutionState::Running]);
    let table = [
        (vec![busy.clone(), agent("docs")], vec![Action::Wait]),
        (
            vec![busy, operation("check", "checks.run")],
            vec![Action::RunOperation {
                step: id("check"),
                op: OperationId("checks.run".into()),
            }],
        ),
    ];

    for (steps, expected) in table {
        let ctx = Ctx {
            free_slots: Vec::new(),
            ..ctx()
        };

        assert_eq!(decide(&root(steps), &ctx), expected);
    }
}

#[test]
fn with_every_slot_held_by_other_drivers_queued_agents_wait_until_a_stop() {
    let table = [
        (vec![task("A", &[])], StopLevel::None, Action::Wait),
        (
            vec![stuck("A"), task("B", &[])],
            StopLevel::None,
            Action::Wait,
        ),
        (
            vec![task("A", &[])],
            StopLevel::Graceful,
            Action::Return(DriveOutcome::Interrupted),
        ),
    ];

    for (items, stop, expected) in table {
        let ctx = Ctx {
            free_slots: Vec::new(),
            stop,
            ..ctx()
        };

        assert_eq!(decide(&root(vec![implement(items)]), &ctx), [expected]);
    }
}
