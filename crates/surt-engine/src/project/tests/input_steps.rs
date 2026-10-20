use super::fixtures::{
    Catalog, ProjectDir, agent, default_config, in_step, new_run, refusal, writer,
};
use crate::project::{Problem, StepField};

const ASKER: &str = r#"
schema = 1
id = "asker"
outcomes = ["done", "questions"]

[inputs.request]
format = "text"

[outputs.spec]
format = "markdown"
outcomes = ["done"]

[outputs.spec-questions]
format = "questions"
outcomes = ["questions"]

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const READER: &str = r#"
schema = 1
id = "reader"

[inputs.spec-answers]
format = "json"

[outputs.notes]
format = "text"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

fn catalog(workflow: &'static str) -> Catalog {
    Catalog::new()
        .workflow("flow", workflow)
        .agent("asker", agent(ASKER))
        .agent("reader", agent(READER))
        .agent("writer", writer())
}

#[tokio::test(flavor = "multi_thread")]
async fn an_input_step_produces_its_answer_artifact_for_the_steps_after_it() {
    let dir = ProjectDir::new();
    let project = dir
        .project(
            catalog(
                r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.ask]
kind = "agent"
agent = "asker"

[steps.answer]
kind = "input"
artifact = "spec-questions"
answer_artifact = "spec-answers"
needs = ["ask"]
when = { step = "ask", outcome = "questions" }

[steps.read]
kind = "agent"
agent = "reader"
needs = ["answer"]
when = { step = "answer", outcome = "answered" }
"#,
            ),
            default_config(),
        )
        .await;

    let created = project.create_run(new_run("flow")).await;

    assert!(created.is_ok(), "the run is created: {created:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_input_step_must_ask_questions_produced_before_it() {
    let unproduced = refusal(catalog(
        r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.answer]
kind = "input"
artifact = "spec-questions"
answer_artifact = "spec-answers"
"#,
    ))
    .await;
    let not_questions = refusal(catalog(
        r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.write]
kind = "agent"
agent = "writer"

[steps.answer]
kind = "input"
artifact = "draft"
answer_artifact = "draft-answers"
needs = ["write"]
"#,
    ))
    .await;

    assert_eq!(
        unproduced,
        in_step(
            "answer",
            Problem::UnproducedArtifact {
                artifact: "spec-questions".to_owned()
            }
        )
    );
    assert_eq!(
        not_questions,
        in_step(
            "answer",
            Problem::NotQuestions {
                artifact: "draft".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_answer_artifact_may_not_reuse_a_rejection_or_answer_artifact() {
    let answer_after_gate = refusal(catalog(
        r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.ask]
kind = "agent"
agent = "asker"

[steps.b-gate]
kind = "approval"
artifact = "spec"
rejection_artifact = "human"
needs = ["ask"]

[steps.c-answer]
kind = "input"
artifact = "spec-questions"
answer_artifact = "human"
needs = ["ask"]
"#,
    ))
    .await;
    let two_answers = refusal(catalog(
        r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.ask]
kind = "agent"
agent = "asker"

[steps.b-answer]
kind = "input"
artifact = "spec-questions"
answer_artifact = "human"
needs = ["ask"]

[steps.c-again]
kind = "input"
artifact = "spec-questions"
answer_artifact = "human"
needs = ["ask"]

[steps.d-gate]
kind = "approval"
artifact = "spec"
rejection_artifact = "human"
needs = ["ask"]
"#,
    ))
    .await;

    assert_eq!(
        answer_after_gate,
        in_step(
            "c-answer",
            Problem::DuplicateAnswerArtifact {
                artifact: "human".to_owned()
            }
        )
    );
    assert_eq!(
        two_answers,
        in_step(
            "c-again",
            Problem::DuplicateAnswerArtifact {
                artifact: "human".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_rejection_artifact_may_not_reuse_an_answer_artifact() {
    let invalid = refusal(catalog(
        r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.ask]
kind = "agent"
agent = "asker"

[steps.b-answer]
kind = "input"
artifact = "spec-questions"
answer_artifact = "human"
needs = ["ask"]

[steps.c-gate]
kind = "approval"
artifact = "spec"
rejection_artifact = "human"
needs = ["ask"]
"#,
    ))
    .await;

    assert_eq!(
        invalid,
        in_step(
            "c-gate",
            Problem::DuplicateRejectionArtifact {
                artifact: "human".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_input_step_has_the_single_outcome_answered() {
    let invalid = refusal(catalog(
        r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.ask]
kind = "agent"
agent = "asker"

[steps.answer]
kind = "input"
artifact = "spec-questions"
answer_artifact = "spec-answers"
needs = ["ask"]

[steps.read]
kind = "agent"
agent = "reader"
needs = ["answer"]
when = { step = "answer", outcome = "approved" }
"#,
    ))
    .await;

    assert_eq!(
        invalid,
        in_step(
            "read",
            Problem::UnknownOutcome {
                field: StepField::When,
                step: "answer".to_owned(),
                outcome: "approved".to_owned(),
                declared: vec!["answered".to_owned()],
            }
        )
    );
}
