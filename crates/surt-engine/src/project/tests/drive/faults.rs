use std::fmt::Debug;
use std::fs;

use super::super::fixtures::{ProjectDir, SUBJECT, default_config, new_run};
use super::fixtures::{ACTOR, Feature, draft, review_catalog, summary};
use super::planned::{TASK_WORKFLOW, approved, work};
use super::recovery::{Orphaning, agent_process, exited};
use crate::project::{Checkpoint, DriveOutcome, Error, Refusal};
use crate::runtime::{Behavior, FakeRuntime, Script};

const DRAFT: &str = "# Draft 1\n";

pub(super) fn assert_crashed<T: Debug>(result: Result<T, Error>, checkpoint: Checkpoint) {
    match result {
        Err(Error::Internal(error)) => {
            assert_eq!(error.to_string(), format!("crashed after {checkpoint:?}"));
        }
        other => panic!("expected a crash after {checkpoint:?}, got {other:?}"),
    }
}

fn written_once() -> Script {
    Script::new()
        .on("write", 1, 1, draft(DRAFT))
        .on("publish", 1, 1, summary("published\n"))
}

impl Feature {
    async fn reaches_the_approved_gate(&self, script: Script) {
        let reached = self.drive(script).await.outcome;
        assert!(
            matches!(reached, Ok(DriveOutcome::AtGate { .. })),
            "{reached:?}"
        );
        self.project
            .approve(&self.run, None, ACTOR)
            .await
            .expect("the draft is approved");
    }

    async fn attempts_of(&self, step: &str) -> Vec<(i64, String)> {
        let sql = format!(
            "SELECT e.attempt, e.state FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             WHERE s.step_key = '{step}' ORDER BY e.attempt"
        );
        self.dir.rows(&sql).await
    }

    async fn finishes(&self, script: Script) {
        let reached = self.drive(script.clone()).await.outcome;
        assert!(
            matches!(reached, Ok(DriveOutcome::AtGate { .. })),
            "{reached:?}"
        );
        let working_copy = self.dir.path().join(".surt/features").join(SUBJECT);
        assert_eq!(
            fs::read_to_string(working_copy.join("draft.md")).expect("the working copy"),
            DRAFT
        );
        self.project
            .approve(&self.run, None, ACTOR)
            .await
            .expect("the draft is approved");
        let finished = self.drive(script).await.outcome;
        assert_eq!(finished.expect("drive returns"), DriveOutcome::Succeeded);
    }

    async fn assert_every_effect_once(&self) {
        let runs: Vec<(String,)> = self.dir.rows("SELECT state FROM workflow_run").await;
        assert_eq!(runs, [("succeeded".to_owned(),)]);
        let revisions: Vec<(String, i64)> = self
            .dir
            .rows(
                "SELECT a.name, count(*) FROM artifact_revision r JOIN artifact a ON a.id = r.artifact_id \
                 WHERE a.name IN ('draft', 'summary') GROUP BY a.name ORDER BY a.name",
            )
            .await;
        assert_eq!(
            revisions,
            [("draft".to_owned(), 1), ("summary".to_owned(), 1)]
        );
        let approvals: Vec<(String,)> = self.dir.rows("SELECT decision FROM approval").await;
        assert_eq!(approvals, [("approve".to_owned(),)]);
        let succeeded: Vec<(String, i64)> = self
            .dir
            .rows(
                "SELECT s.step_key, count(*) FROM execution e JOIN step_run s ON s.id = e.step_run_id \
                 WHERE e.state = 'succeeded' GROUP BY s.step_key ORDER BY s.step_key",
            )
            .await;
        assert_eq!(
            succeeded,
            [("publish".to_owned(), 1), ("write".to_owned(), 1)]
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_right_after_the_run_is_created_leaves_one_run_that_the_next_drive_finishes() {
    let dir = ProjectDir::new();
    let project = dir.project(review_catalog(), default_config()).await;
    project.crash_after(Checkpoint::RunCreated);

    assert_crashed(
        project.create_run(new_run("review")).await,
        Checkpoint::RunCreated,
    );

    let again = project.create_run(new_run("review")).await;
    assert!(
        matches!(again, Err(Error::Refused(Refusal::ActiveRunExists { .. }))),
        "{again:?}"
    );
    let run = project
        .resolve_subject(SUBJECT)
        .await
        .expect("the subject exists")
        .active_run
        .expect("its run is active");
    let feature = Feature { dir, project, run };
    feature.finishes(written_once()).await;
    feature.assert_every_effect_once().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_right_after_an_iteration_starts_resumes_that_iteration_without_opening_another() {
    let feature = Feature::review().await;
    feature
        .project
        .crash_after(Checkpoint::IterationOrExpansion);

    assert_crashed(
        feature.drive(written_once()).await.outcome,
        Checkpoint::IterationOrExpansion,
    );

    feature.finishes(written_once()).await;
    feature.assert_every_effect_once().await;
    let iterations: Vec<(i64, String)> = feature
        .dir
        .rows("SELECT iteration, state FROM step_run WHERE step_key = 'write' ORDER BY iteration")
        .await;
    assert_eq!(iterations, [(1, "done".to_owned())]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_right_after_the_items_expand_resumes_each_child_run_once() {
    let feature = approved(TASK_WORKFLOW, &[("A", &[]), ("B", &["A"])]).await;
    let script = Script::new()
        .on("A/build", 1, 1, work("a"))
        .on("B/build", 1, 1, work("b"));
    feature
        .project
        .crash_after(Checkpoint::IterationOrExpansion);

    assert_crashed(
        feature.drive(script.clone()).await.outcome,
        Checkpoint::IterationOrExpansion,
    );

    let resumed = feature.drive(script).await.outcome;
    assert_eq!(resumed.expect("drive returns"), DriveOutcome::Succeeded);
    let children: Vec<(String, String)> = feature
        .dir
        .rows(
            "SELECT item_key, state FROM workflow_run WHERE parent_step_run_id IS NOT NULL \
             ORDER BY item_key",
        )
        .await;
    assert_eq!(
        children,
        [
            ("A".to_owned(), "succeeded".to_owned()),
            ("B".to_owned(), "succeeded".to_owned()),
        ]
    );
    let builds: Vec<(i64,)> = feature
        .dir
        .rows(
            "SELECT e.attempt FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             WHERE s.step_key = 'build' ORDER BY e.attempt",
        )
        .await;
    assert_eq!(builds, [(1,), (1,)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_right_after_an_execution_starts_keeps_the_approval_and_retries_the_attempt() {
    let feature = Feature::review().await;
    let script = written_once().on("publish", 1, 2, summary("published\n"));
    feature.reaches_the_approved_gate(script.clone()).await;
    feature.project.crash_after(Checkpoint::ExecutionStarted);

    assert_crashed(
        feature.drive(script.clone()).await.outcome,
        Checkpoint::ExecutionStarted,
    );

    let resumed = feature.drive(script).await.outcome;
    assert_eq!(resumed.expect("drive returns"), DriveOutcome::Succeeded);
    feature.assert_every_effect_once().await;
    assert_eq!(
        feature.attempts_of("publish").await,
        [(1, "interrupted".to_owned()), (2, "succeeded".to_owned())]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_right_after_a_process_is_recorded_stops_that_process_and_retries_the_attempt() {
    let feature = Feature::review().await;
    let mut orphan = agent_process();
    let orphaning = Orphaning {
        fake: FakeRuntime::new(written_once()),
        pid: orphan.id(),
    };
    feature.project.crash_after(Checkpoint::ProcessSpawned);

    assert_crashed(
        feature.drive_with(&orphaning).await.outcome,
        Checkpoint::ProcessSpawned,
    );
    assert!(orphan.try_wait().expect("inspectable").is_none());

    feature
        .finishes(written_once().on("write", 1, 2, draft(DRAFT)))
        .await;
    assert!(exited(&mut orphan), "the orphan was stopped");
    feature.assert_every_effect_once().await;
    assert_eq!(
        feature.attempts_of("write").await,
        [(1, "interrupted".to_owned()), (2, "succeeded".to_owned())]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_right_after_an_execution_is_accepted_writes_its_working_copy_on_the_next_drive() {
    let feature = Feature::review().await;
    feature.project.crash_after(Checkpoint::ExecutionAccepted);

    assert_crashed(
        feature.drive(written_once()).await.outcome,
        Checkpoint::ExecutionAccepted,
    );

    feature.finishes(written_once()).await;
    feature.assert_every_effect_once().await;
    assert_eq!(
        feature.attempts_of("write").await,
        [(1, "succeeded".to_owned())]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_right_after_the_working_copy_is_written_records_no_edit_and_no_new_revision() {
    let feature = Feature::review().await;
    feature.project.crash_after(Checkpoint::WorkingCopyWritten);

    assert_crashed(
        feature.drive(written_once()).await.outcome,
        Checkpoint::WorkingCopyWritten,
    );

    feature.finishes(written_once()).await;
    feature.assert_every_effect_once().await;
    let origins: Vec<(String,)> = feature
        .dir
        .rows(
            "SELECT r.origin FROM artifact_revision r JOIN artifact a ON a.id = r.artifact_id \
             WHERE a.name = 'draft'",
        )
        .await;
    assert_eq!(origins, [("execution".to_owned(),)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_right_after_a_failed_execution_ends_spends_that_attempt_only_once() {
    let feature = Feature::review().await;
    let script = Script::new()
        .on("write", 1, 1, Behavior::Crash { exit_code: 1 })
        .on("write", 1, 2, draft(DRAFT))
        .on("publish", 1, 1, summary("published\n"));
    feature.project.crash_after(Checkpoint::ExecutionEnded);

    assert_crashed(
        feature.drive(script.clone()).await.outcome,
        Checkpoint::ExecutionEnded,
    );

    feature.finishes(script).await;
    feature.assert_every_effect_once().await;
    assert_eq!(
        feature.attempts_of("write").await,
        [(1, "failed".to_owned()), (2, "succeeded".to_owned())]
    );
}
