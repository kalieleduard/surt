use std::fs;
use std::path::PathBuf;

use super::super::fixtures::SUBJECT;
use super::fixtures::{ACTOR, Feature, Recording, draft, label, summary};
use super::pinning::Editing;
use crate::project::{Decided, DriveOutcome, Gate, GateStatus, Progress, Refusal, RevisionOrigin};
use crate::runtime::Script;

const EDITED: &str = "# Draft 1, edited by hand\n";

impl Feature {
    pub(super) fn working_copy(&self, rel_path: &str) -> PathBuf {
        self.dir
            .path()
            .join(".surt/features")
            .join(SUBJECT)
            .join(rel_path)
    }

    pub(super) fn edit(&self, rel_path: &str, text: &str) {
        let path = self.working_copy(rel_path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        fs::write(path, text).expect("the edit is written");
    }
}

pub(super) async fn at_gate() -> Feature {
    let feature = Feature::review().await;
    feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await
        .outcome
        .expect("the draft reaches the gate");
    feature
}

async fn revisions(feature: &Feature) -> Vec<(i64, String, Option<String>, Vec<u8>)> {
    feature
        .dir
        .rows(
            "SELECT r.number, r.origin, r.human_action, b.bytes FROM artifact_revision r \
             JOIN artifact a ON a.id = r.artifact_id JOIN revision_file f ON f.revision_id = r.id \
             JOIN blob b ON b.sha256 = f.blob_sha256 WHERE a.name = 'draft' ORDER BY r.number",
        )
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn an_edit_becomes_a_human_revision_and_the_approval_targets_it() {
    let feature = at_gate().await;
    feature.edit("draft.md", EDITED);

    let approved = feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the approval is recorded");

    let edit: Vec<(String,)> = feature
        .dir
        .rows("SELECT identity_sha256 FROM artifact_revision WHERE number = 2")
        .await;
    assert_eq!(
        approved,
        Decided {
            gate: "approve".to_owned(),
            artifact: "draft".to_owned(),
            revision: 2,
            sha256: edit[0].0.clone(),
            origin: RevisionOrigin::Human,
            actor: ACTOR.to_owned(),
            files: vec!["draft.md".to_owned()],
            restored: Vec::new(),
        }
    );
    assert_eq!(
        revisions(&feature).await,
        [
            (1, "execution".to_owned(), None, b"# Draft 1\n".to_vec()),
            (
                2,
                "human".to_owned(),
                Some("edit".to_owned()),
                EDITED.as_bytes().to_vec()
            ),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_rejection_records_the_edit_first_and_rejects_it() {
    let feature = at_gate().await;
    feature.edit("draft.md", EDITED);

    let rejected = feature
        .project
        .reject(&feature.run, Some(2), "shorter", ACTOR)
        .await
        .expect("the rejection is recorded");

    assert_eq!(
        (rejected.revision, rejected.origin),
        (2, RevisionOrigin::Human)
    );
    let decisions: Vec<(String, i64)> = feature
        .dir
        .rows(
            "SELECT a.decision, r.number FROM approval a \
             JOIN artifact_revision r ON r.id = a.revision_id",
        )
        .await;
    assert_eq!(decisions, [("reject".to_owned(), 2)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn edits_are_detected_by_content_never_by_modification_time() {
    let unchanged = at_gate().await;
    let path = unchanged.working_copy("draft.md");
    fs::write(&path, "# Draft 1\n").expect("the same bytes are rewritten");
    let touched = unchanged
        .project
        .approve(&unchanged.run, None, ACTOR)
        .await
        .expect("the approval is recorded");
    let same_time = at_gate().await;
    let path = same_time.working_copy("draft.md");
    let modified = fs::metadata(&path)
        .and_then(|metadata| metadata.modified())
        .expect("the modification time");
    fs::write(&path, "# Draft 2\n").expect("the edit is written");
    fs::File::options()
        .write(true)
        .open(&path)
        .and_then(|file| file.set_modified(modified))
        .expect("the modification time is restored");

    let edited = same_time
        .project
        .approve(&same_time.run, None, ACTOR)
        .await
        .expect("the approval is recorded");

    assert_eq!(
        (touched.revision, touched.origin),
        (1, RevisionOrigin::Agent)
    );
    assert_eq!((edited.revision, edited.origin), (2, RevisionOrigin::Human));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_deleted_file_is_recreated_from_the_latest_revision_by_a_decision() {
    let feature = at_gate().await;
    fs::remove_file(feature.working_copy("draft.md")).expect("the file is deleted");

    let approved = feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the approval is recorded");

    assert_eq!(approved.restored, ["draft.md"]);
    assert_eq!(
        (approved.revision, approved.origin),
        (1, RevisionOrigin::Agent)
    );
    assert_eq!(
        fs::read_to_string(feature.working_copy("draft.md")).expect("the recreated file"),
        "# Draft 1\n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_deleted_file_is_recreated_by_a_driving_command_and_reported() {
    let feature = at_gate().await;
    fs::remove_dir_all(feature.working_copy("")).expect("the Working Copy is deleted");

    let driven = feature.drive(Script::new()).await;

    assert!(matches!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::AtGate { .. }
    ));
    assert_eq!(
        driven.progress,
        [Progress::FileRestored {
            artifact: "draft".to_owned(),
            path: "draft.md".to_owned(),
            revision: 1,
        }]
    );
    assert_eq!(
        fs::read_to_string(feature.working_copy("draft.md")).expect("the recreated file"),
        "# Draft 1\n"
    );
    assert_eq!(revisions(&feature).await.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_edit_after_approval_is_unapproved_and_nothing_drives_until_it_is_approved() {
    let feature = at_gate().await;
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the agent draft is approved");
    feature.edit("draft.md", EDITED);
    let runtime = Recording::new(Script::new().on("publish", 1, 1, summary("ok\n")));

    let halted = feature.drive_with(&runtime).await;
    let launched_while_unapproved = runtime.requests().len();
    let approved = feature
        .project
        .approve(&feature.run, Some(2), ACTOR)
        .await
        .expect("the edit is approved");
    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        halted.outcome.expect("drive returns"),
        DriveOutcome::Halted(Refusal::UnapprovedRevision {
            subject: SUBJECT.to_owned(),
            artifact: "draft".to_owned(),
            revision: 2,
        })
    );
    assert_eq!(launched_while_unapproved, 0);
    assert_eq!(
        (approved.gate.as_str(), approved.revision, approved.origin),
        ("approve", 2, RevisionOrigin::Human)
    );
    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let read: Vec<(i64, bool)> = feature
        .dir
        .rows(
            "SELECT r.number, r.pinned_at IS NOT NULL FROM execution_input i \
             JOIN artifact_revision r ON r.id = i.revision_id WHERE i.input_name = 'draft'",
        )
        .await;
    assert_eq!(read, [(2, true)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_revision_over_an_unrecorded_edit_saves_the_edit_first_and_warns() {
    let feature = at_gate().await;
    feature
        .project
        .reject(&feature.run, None, "shorter", ACTOR)
        .await
        .expect("the rejection is recorded");
    let runtime = Editing::new(
        Script::new().on("write", 2, 1, draft("# Draft 2\n")),
        "write",
        feature.working_copy("draft.md"),
        EDITED,
    );

    let driven = feature.drive_with(&runtime).await;

    assert!(matches!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::AtGate { .. }
    ));
    assert!(
        driven.progress.contains(&Progress::EditSuperseded {
            artifact: "draft".to_owned(),
            edit: 2,
            by: 3,
        }),
        "{:?}",
        driven.progress
    );
    assert_eq!(
        revisions(&feature).await,
        [
            (1, "execution".to_owned(), None, b"# Draft 1\n".to_vec()),
            (
                2,
                "human".to_owned(),
                Some("edit".to_owned()),
                EDITED.as_bytes().to_vec()
            ),
            (3, "execution".to_owned(), None, b"# Draft 2\n".to_vec()),
        ]
    );
    assert_eq!(
        fs::read_to_string(feature.working_copy("draft.md")).expect("the Working Copy"),
        "# Draft 2\n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn only_artifacts_behind_an_approval_gate_have_a_working_copy() {
    let feature = at_gate().await;
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the draft is approved");

    let driven = feature
        .drive(Script::new().on("publish", 1, 1, summary("ok\n")))
        .await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let files: Vec<(String, String)> = feature
        .dir
        .rows(
            "SELECT a.name, w.rel_path FROM working_copy_file w \
             JOIN artifact a ON a.id = w.artifact_id ORDER BY a.name",
        )
        .await;
    assert_eq!(files, [("draft".to_owned(), "draft.md".to_owned())]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_driving_command_records_the_edit_before_it_drives() {
    let feature = at_gate().await;
    feature.edit("draft.md", EDITED);

    let driven = feature.drive(Script::new()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::AtGate {
            gates: vec![Gate {
                step: label("approve", Some(1)),
                artifact: "draft".to_owned(),
            }]
        }
    );
    assert_eq!(
        driven.progress,
        [Progress::EditRecorded {
            artifact: "draft".to_owned(),
            revision: 2,
        }]
    );
    assert_eq!(
        feature
            .project
            .gate_status(&feature.run, "approve")
            .await
            .expect("the gate status"),
        GateStatus::Waiting { revision: 2 }
    );
}
