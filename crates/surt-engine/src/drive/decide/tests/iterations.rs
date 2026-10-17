use super::super::{Action, decide};
use super::fixtures::{agent, approval, ctx, id, launch, looping, operation, root};
use crate::drive::graph::{BlockReason, Step};

fn start_iteration(loop_: &str) -> Action {
    Action::StartIteration { loop_: id(loop_) }
}

fn rejected_cycle() -> Vec<Step> {
    vec![
        agent("build").done("done"),
        operation("integrate", "git.integrate")
            .needs(&["build"])
            .done("conflict"),
    ]
}

fn rejected_spec() -> Vec<Step> {
    vec![
        agent("spec").done("done"),
        approval("approve-spec").needs(&["spec"]).done("rejected"),
    ]
}

fn cycle(iterations: usize) -> Step {
    looping(
        "cycle",
        ("integrate", "merged"),
        vec![rejected_cycle(); iterations],
    )
}

#[test]
fn a_loop_whose_needs_finished_starts_its_first_iteration() {
    let graph = root(vec![
        agent("setup").done("done"),
        cycle(0).needs(&["setup"]),
    ]);

    assert_eq!(decide(&graph, &ctx()), vec![start_iteration("cycle")]);
}

#[test]
fn the_steps_of_the_current_iteration_are_scheduled_within_it() {
    let current = vec![agent("build"), agent("review").needs(&["build"])];
    let graph = root(vec![
        looping(
            "cycle",
            ("review", "approved"),
            vec![rejected_cycle(), current],
        )
        .running(),
    ]);

    assert_eq!(decide(&graph, &ctx()), vec![launch("cycle#2/build")]);
}

#[test]
fn an_iteration_that_matches_until_finishes_the_loop() {
    let mut merged = rejected_cycle();
    merged[1] = merged[1].clone().done("merged");
    let graph = root(vec![
        looping("cycle", ("integrate", "merged"), vec![merged]).running(),
    ]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::FinishStep { step: id("cycle") }]
    );
}

#[test]
fn iterations_repeat_until_max_iterations_then_the_loop_blocks() {
    let table = [
        (1, start_iteration("cycle")),
        (4, start_iteration("cycle")),
        (
            5,
            Action::Block {
                step: id("cycle"),
                reason: BlockReason::IterationsExhausted,
            },
        ),
    ];

    for (iterations, expected) in table {
        let graph = root(vec![cycle(iterations).running()]);

        assert_eq!(decide(&graph, &ctx()), vec![expected], "{iterations}");
    }
}

#[test]
fn a_loop_containing_an_approval_gate_has_no_iteration_limit() {
    let spec_loop = looping(
        "spec-loop",
        ("approve-spec", "approved"),
        vec![rejected_spec(); 12],
    );
    let graph = root(vec![spec_loop.running()]);

    assert_eq!(decide(&graph, &ctx()), vec![start_iteration("spec-loop")]);
}

#[test]
fn unblocking_a_loop_renews_its_iteration_budget_and_keeps_the_history() {
    let table = [
        (5, 5, start_iteration("cycle")),
        (9, 5, start_iteration("cycle")),
        (
            10,
            5,
            Action::Block {
                step: id("cycle"),
                reason: BlockReason::IterationsExhausted,
            },
        ),
    ];

    for (iterations, budget_start, expected) in table {
        let graph = root(vec![cycle(iterations).running().budget_start(budget_start)]);

        assert_eq!(decide(&graph, &ctx()), vec![expected], "{iterations}");
    }
}
