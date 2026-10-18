use std::fs;

use super::super::fixtures::SUBJECT;
use super::super::refusal;
use super::fixtures::{ACTOR, Feature, PUBLISHER, WRITER, agent, draft, summary};
use crate::project::{Catalog, DriveOutcome, Progress, Refusal};
use crate::runtime::{
    AgentRuntime, Behavior, Delivery, FakeRuntime, RuntimeCompletion, RuntimeError,
    RuntimeEventSink, RuntimeRequest, Script, StopSignal,
};

const PINNING_WORKFLOW: &str = r#"
schema = 1
name = "pinning"

[steps.write]
kind = "agent"
agent = "writer"

[steps.approve]
kind = "approval"
artifact = "draft"
rejection_artifact = "draft-rejection"
needs = ["write"]

[steps.drafting]
kind = "loop"
body = ["write", "approve"]
until = { step = "approve", outcome = "approved" }

[steps.publish]
kind = "agent"
agent = "publisher"
needs = ["drafting"]

[steps.archive]
kind = "agent"
agent = "archiver"
needs = ["publish"]
"#;

const ARCHIVER: &str = r#"
schema = 1
id = "archiver"

[inputs.summary]
format = "text"

[outputs.archive]
format = "text"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const EDITED: &str = "# Draft 1, edited after publishing\n";

pub(super) struct Editing {
    fake: FakeRuntime,
    step: &'static str,
    file: std::path::PathBuf,
    text: &'static str,
}

impl Editing {
    pub(super) fn new(
        script: Script,
        step: &'static str,
        file: std::path::PathBuf,
        text: &'static str,
    ) -> Editing {
        Editing {
            fake: FakeRuntime::new(script),
            step,
            file,
            text,
        }
    }
}

impl AgentRuntime for Editing {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        if request.step.step == self.step {
            fs::write(&self.file, self.text).expect("the edit is written while the agent runs");
        }
        self.fake.execute(request, events, stop).await
    }
}

fn archive() -> Behavior {
    Behavior::Deliver(Delivery::new("done").output("archive", "archive.txt", "kept\n"))
}

async fn approved_draft() -> Feature {
    let catalog = Catalog::new()
        .workflow("pinning", PINNING_WORKFLOW)
        .agent("writer", agent(WRITER))
        .agent("publisher", agent(PUBLISHER))
        .agent("archiver", agent(ARCHIVER));
    let feature = Feature::new(catalog, "pinning").await;
    feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await
        .outcome
        .expect("the draft reaches the gate");
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the draft is approved");
    feature
}

fn pinned_edit() -> Refusal {
    Refusal::PinnedEdit {
        subject: SUBJECT.to_owned(),
        artifact: "draft".to_owned(),
        revision: 1,
        files: vec!["draft.md".to_owned()],
    }
}

async fn executions(feature: &Feature) -> Vec<(String, String)> {
    feature
        .dir
        .rows(
            "SELECT s.step_key, e.state FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             ORDER BY s.step_key",
        )
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn an_edit_to_a_pinned_revision_lets_running_work_finish_and_launches_nothing_new() {
    let feature = approved_draft().await;
    let script = Script::new()
        .on("publish", 1, 1, summary("ok\n"))
        .on("archive", 1, 1, archive());
    let runtime = Editing::new(
        script.clone(),
        "publish",
        feature.working_copy("draft.md"),
        EDITED,
    );

    let halted = feature.drive_with(&runtime).await;
    let approve = feature.project.approve(&feature.run, None, ACTOR).await;

    assert_eq!(
        halted.outcome.expect("drive returns"),
        DriveOutcome::Halted(pinned_edit())
    );
    assert_eq!(refusal(approve), pinned_edit());
    assert_eq!(
        executions(&feature).await,
        [
            ("publish".to_owned(), "succeeded".to_owned()),
            ("write".to_owned(), "succeeded".to_owned()),
        ]
    );
    let revisions: Vec<(i64,)> = feature
        .dir
        .rows(
            "SELECT count(*) FROM artifact_revision r JOIN artifact a ON a.id = r.artifact_id \
             WHERE a.name = 'draft'",
        )
        .await;
    assert_eq!(revisions, [(1,)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn removing_the_edited_file_of_a_pinned_revision_lets_the_run_continue() {
    let feature = approved_draft().await;
    let script = Script::new()
        .on("publish", 1, 1, summary("ok\n"))
        .on("archive", 1, 1, archive());
    let runtime = Editing::new(
        script.clone(),
        "publish",
        feature.working_copy("draft.md"),
        EDITED,
    );
    feature.drive_with(&runtime).await;
    fs::remove_file(feature.working_copy("draft.md")).expect("the edited file is removed");

    let driven = feature.drive(script).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert!(driven.progress.contains(&Progress::FileRestored {
        artifact: "draft".to_owned(),
        path: "draft.md".to_owned(),
        revision: 1,
    }));
}
