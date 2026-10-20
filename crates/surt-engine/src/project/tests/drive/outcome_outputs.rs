use super::super::fixtures::Catalog;
use super::fixtures::{Feature, agent, label};
use crate::project::{DriveOutcome, Progress};
use crate::runtime::{Behavior, Delivery, Script};

const FLOW: &str = r#"
schema = 1
name = "ask"

[inputs.request]
format = "text"

[steps.write]
kind = "agent"
agent = "asker"
"#;

const ASKER: &str = r#"
schema = 1
id = "asker"
outcomes = ["done", "questions"]

[inputs.request]
format = "text"

[outputs.commit]
format = "json"
outcomes = ["done"]
checks = [{ kind = "commit_evidence" }]

[outputs.spec-questions]
format = "json"
outcomes = ["questions"]

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

#[tokio::test(flavor = "multi_thread")]
async fn an_outcome_publishes_only_its_outputs_and_skips_the_checks_of_the_others() {
    let catalog = Catalog::new()
        .workflow("ask", FLOW)
        .agent("asker", agent(ASKER));
    let feature = Feature::new(catalog, "ask").await;
    let asked = Delivery::new("questions").output(
        "spec-questions",
        "questions.json",
        r#"{ "questions": [] }"#,
    );

    let driven = feature
        .drive(Script::new().on("write", 1, 1, Behavior::Deliver(asked)))
        .await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(
        driven.progress[..3],
        [
            Progress::StepStarted {
                step: label("write", None),
                attempt: 1,
            },
            Progress::RevisionPublished {
                artifact: "spec-questions".to_owned(),
                revision: 1,
            },
            Progress::StepFinished {
                step: label("write", None),
                outcome: "questions".to_owned(),
            },
        ]
    );
}
