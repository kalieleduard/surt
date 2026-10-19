use std::fs;

use super::super::fixtures::{SUBJECT, initial};
use super::fixtures::{ACTOR, Feature};
use super::planned::plan;
use super::working_copy::at_gate;
use super::worktrees::{refused, tracked};
use crate::project::{Config, DriveOutcome, Error, NewRun, Refusal, RunId};
use crate::runtime::Script;

const REQUEST: &str = "request";

fn next_run(feature: &Feature, workflow: &str) -> NewRun {
    let base = feature.base();
    let base_json = format!(r#"{{"ref": "main", "sha": "{base}"}}"#);
    NewRun {
        subject: SUBJECT.to_owned(),
        workflow: workflow.to_owned(),
        initial_artifacts: vec![initial("base", "base.json", base_json.as_bytes())],
        carried: vec![REQUEST.to_owned()],
        merge_into_base: None,
        actor: ACTOR.to_owned(),
    }
}

fn review_next_run() -> NewRun {
    NewRun {
        subject: SUBJECT.to_owned(),
        workflow: "review".to_owned(),
        initial_artifacts: Vec::new(),
        carried: vec![REQUEST.to_owned()],
        merge_into_base: None,
        actor: ACTOR.to_owned(),
    }
}

async fn cancelled_with_a_blocked_task() -> Feature {
    let feature = tracked(&[("A", &[])], Config::default()).await;
    let blocked = feature
        .drive(Script::new().on("A/build", 1, 1, refused()))
        .await
        .outcome
        .expect("drive returns");
    assert!(
        matches!(blocked, DriveOutcome::Blocked { .. }),
        "{blocked:?}"
    );
    feature.cancel().await.outcome.expect("run 1 is cancelled");
    feature
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_run_carries_the_request_and_works_on_dot_two_branches_and_worktrees() {
    let mut feature = cancelled_with_a_blocked_task().await;
    let first_task = format!("refs/heads/surt/task/{SUBJECT}/A");
    let first_tip = feature.sha(&first_task);
    let first_integration = feature.sha(&format!("refs/heads/surt/{SUBJECT}"));

    feature.run = feature
        .project
        .create_run(next_run(&feature, "tracked"))
        .await
        .expect("run 2 is created")
        .run;
    feature
        .drive(Script::new().on("plan", 1, 1, plan(&[("A", &[])])))
        .await
        .outcome
        .expect("the plan reaches its gate");
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the plan is approved");
    let driven = feature
        .drive(Script::new().on("A/build", 1, 1, refused()))
        .await;

    assert!(matches!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Blocked { .. }
    ));
    let stem = format!("{SUBJECT}.2");
    let worktree = feature
        .worktree("A")
        .parent()
        .and_then(|subject| subject.parent())
        .expect("the project's worktrees")
        .join(&stem)
        .join("A");
    let branches: Vec<(String, Option<String>)> = feature
        .dir
        .rows(&format!(
            "SELECT b.ref_name, b.worktree_path FROM run_branch b \
             JOIN workflow_run w ON w.id = b.workflow_run_id WHERE w.root_run_id = '{}' \
             ORDER BY b.ref_name",
            feature.run.as_str()
        ))
        .await;
    assert_eq!(
        branches,
        [
            (format!("refs/heads/surt/{stem}"), None),
            (
                format!("refs/heads/surt/task/{stem}/A"),
                Some(worktree.display().to_string())
            ),
        ]
    );
    assert!(worktree.is_dir());
    assert_eq!(feature.sha(&first_task), first_tip);
    assert_eq!(
        feature.sha(&format!("refs/heads/surt/{SUBJECT}")),
        first_integration
    );
    let requests: Vec<(i64, String, Option<String>, Vec<u8>)> = feature
        .dir
        .rows(
            "SELECT w.ordinal, r.origin, r.human_action, b.bytes FROM artifact_revision r \
             JOIN artifact a ON a.id = r.artifact_id JOIN workflow_run w ON w.id = a.workflow_run_id \
             JOIN revision_file f ON f.revision_id = r.id JOIN blob b ON b.sha256 = f.blob_sha256 \
             WHERE a.name = 'request' ORDER BY w.ordinal",
        )
        .await;
    let first = requests[0].clone();
    assert_eq!(requests, [first.clone(), (2, first.1, first.2, first.3)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_run_is_refused_while_the_subject_has_an_active_run() {
    let feature = Feature::review().await;
    feature.edit("draft.md", "typed while it runs\n");

    let created = feature.project.create_run(review_next_run()).await;

    assert!(
        matches!(&created, Err(Error::Refused(Refusal::ActiveRunExists { subject })) if subject == SUBJECT),
        "{created:?}"
    );
    let runs: Vec<(i64,)> = feature.dir.rows("SELECT count(*) FROM workflow_run").await;
    assert_eq!(runs, [(1,)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_run_is_refused_while_the_previous_working_copy_holds_an_unrecorded_edit() {
    let feature = at_gate().await;
    feature.edit("draft.md", "# Draft 1, edited by hand\n");
    feature.cancel().await.outcome.expect("run 1 is cancelled");

    let created = feature.project.create_run(review_next_run()).await;

    assert_eq!(
        created.map(|_| ()).map_err(|error| match error {
            Error::Refused(refusal) => Some(refusal),
            Error::Internal(_) => None,
        }),
        Err(Some(Refusal::UnrecordedEditsBlockNewRun {
            subject: SUBJECT.to_owned(),
            run: 1,
            files: vec!["draft.md".to_owned()],
        }))
    );
    let runs: Vec<(i64,)> = feature.dir.rows("SELECT count(*) FROM workflow_run").await;
    assert_eq!(runs, [(1,)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn once_the_edits_are_moved_away_the_new_run_starts_from_the_request() {
    let feature = at_gate().await;
    feature.edit("draft.md", "# Draft 1, edited by hand\n");
    feature.cancel().await.outcome.expect("run 1 is cancelled");
    fs::remove_file(feature.working_copy("draft.md")).expect("the edit is moved away");

    let created: RunId = feature
        .project
        .create_run(review_next_run())
        .await
        .expect("run 2 is created")
        .run;

    let runs: Vec<(String, i64, String)> = feature
        .dir
        .rows("SELECT id, ordinal, state FROM workflow_run ORDER BY ordinal")
        .await;
    assert_eq!(runs[1], (created.0, 2, "active".to_owned()));
    let requests: Vec<(Vec<u8>,)> = feature
        .dir
        .rows(
            "SELECT b.bytes FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
             JOIN revision_file f ON f.revision_id = r.id JOIN blob b ON b.sha256 = f.blob_sha256 \
             WHERE a.name = 'request' ORDER BY r.rowid",
        )
        .await;
    assert_eq!(
        requests,
        [(b"add login".to_vec(),), (b"add login".to_vec(),)]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn carrying_an_artifact_of_a_subject_without_runs_is_refused() {
    let feature = Feature::review().await;
    let mut other = review_next_run();
    other.subject = "20261017-elsewhere".to_owned();

    let created = feature.project.create_run(other).await;

    assert!(
        matches!(&created, Err(Error::Refused(Refusal::UnknownSubject { reference })) if reference == "20261017-elsewhere"),
        "{created:?}"
    );
}
