use super::super::{Action, DriveOutcome, decide};
use super::fixtures::{agent, approval, ctx, id, input, looping, root};
use crate::drive::graph::{Step, StepRunState};

fn asked() -> Vec<Step> {
    vec![
        agent("spec").done("questions"),
        input("answer-spec")
            .needs(&["spec"])
            .when("spec", "questions")
            .done("answered"),
        approval("approve-spec")
            .needs(&["answer-spec"])
            .when("spec", "done")
            .in_state(StepRunState::Skipped),
    ]
}

#[test]
fn a_pending_input_step_whose_question_was_asked_opens_and_waits() {
    let graph = root(vec![
        agent("spec").done("questions"),
        input("answer-spec")
            .needs(&["spec"])
            .when("spec", "questions"),
    ]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::OpenGate {
            step: id("answer-spec")
        }]
    );
}

#[test]
fn a_run_waiting_only_on_an_input_step_returns_at_it() {
    let graph = root(vec![
        agent("spec").done("questions"),
        input("answer-spec").needs(&["spec"]).running(),
        agent("plan").needs(&["answer-spec"]),
    ]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::Return(DriveOutcome::AtGate {
            gates: vec![id("answer-spec")]
        })]
    );
}

#[test]
fn a_delivered_output_skips_the_input_step() {
    let graph = root(vec![
        agent("spec").done("done"),
        input("answer-spec")
            .needs(&["spec"])
            .when("spec", "questions"),
    ]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::Skip {
            step: id("answer-spec")
        }]
    );
}

#[test]
fn a_loop_whose_body_holds_only_an_input_step_has_no_iteration_limit() {
    let body = vec![
        agent("spec").done("questions"),
        input("answer-spec")
            .needs(&["spec"])
            .when("spec", "questions")
            .done("answered"),
    ];
    let spec_loop = looping("spec-loop", ("spec", "done"), vec![body; 12]);

    let graph = root(vec![spec_loop.running()]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::StartIteration {
            loop_: id("spec-loop")
        }]
    );
}

#[test]
fn an_answered_round_starts_the_next_iteration_without_reaching_the_gate() {
    let spec_loop = looping("spec-loop", ("approve-spec", "approved"), vec![asked()]);

    let graph = root(vec![spec_loop.running()]);

    assert_eq!(
        decide(&graph, &ctx()),
        vec![Action::StartIteration {
            loop_: id("spec-loop")
        }]
    );
}
