use std::fs;

use serde_json::json;

use super::super::fixtures::SUBJECT;
use super::super::refusal;
use super::faults::assert_crashed;
use super::fixtures::{ACTOR, Feature, draft, summary};
use super::pinning::{Editing, approved_draft};
use super::planned::{TASK_WORKFLOW, plan, planned_catalog};
use crate::platform;
use crate::project::{Change, Checkpoint, DriveOutcome, FileChange, Refusal, RestoreImpact};
use crate::runtime::Script;

const DRAFT: &str = "# Draft 1\n";
const EDITED: &str = "# Draft 1, edited after publishing\n";

fn changed(path: &str, added: usize, removed: usize) -> FileChange {
    FileChange {
        path: path.to_owned(),
        change: Change::Changed { added, removed },
    }
}

fn file(path: &str, change: Change) -> FileChange {
    FileChange {
        path: path.to_owned(),
        change,
    }
}

impl Feature {
    async fn count(&self, sql: &str) -> i64 {
        let rows: Vec<(i64,)> = self.dir.rows(sql).await;
        rows.first().map(|(count,)| *count).unwrap_or_default()
    }

    async fn revisions(&self) -> i64 {
        self.count("SELECT count(*) FROM artifact_revision").await
    }

    async fn approvals(&self) -> i64 {
        self.count("SELECT count(*) FROM approval").await
    }

    async fn pins(&self) -> i64 {
        self.count("SELECT count(*) FROM artifact_revision WHERE pinned_at IS NOT NULL")
            .await
    }

    fn read(&self, rel_path: &str) -> Option<String> {
        fs::read_to_string(self.working_copy(rel_path)).ok()
    }
}

async fn pinned_and_edited() -> (Feature, Script) {
    let feature = approved_draft().await;
    let script = Script::new().on("publish", 1, 1, summary("ok\n")).on(
        "archive",
        1,
        1,
        super::pinning::archive(),
    );
    let runtime = Editing::new(
        script.clone(),
        "publish",
        feature.working_copy("draft.md"),
        EDITED,
    );
    let halted = feature.drive_with(&runtime).await.outcome;
    assert!(
        matches!(halted, Ok(DriveOutcome::Halted(Refusal::PinnedEdit { .. }))),
        "{halted:?}"
    );
    (feature, script)
}

#[tokio::test(flavor = "multi_thread")]
async fn restoring_a_pinned_edit_rewrites_the_file_without_a_revision_and_the_run_continues() {
    let (feature, script) = pinned_and_edited().await;
    let (revisions, approvals, pins) = (
        feature.revisions().await,
        feature.approvals().await,
        feature.pins().await,
    );

    let impact = feature.project.restore_impact(SUBJECT, "draft").await;
    let restored = feature.project.restore(SUBJECT, "draft").await;

    let expected = RestoreImpact {
        artifact: "draft".to_owned(),
        revision: 1,
        changes: vec![changed("draft.md", 1, 1)],
    };
    assert_eq!(impact.expect("the impact is read"), expected);
    assert_eq!(restored.expect("the draft is restored"), expected);
    assert_eq!(feature.read("draft.md").as_deref(), Some(DRAFT));
    assert_eq!(feature.revisions().await, revisions);
    assert_eq!(feature.approvals().await, approvals);
    assert_eq!(feature.pins().await, pins);
    assert_eq!(
        feature.dir.event_data("working_copy.restored").await,
        [json!({
            "artifact": "draft",
            "revision": 1,
            "changed": ["draft.md"],
            "new": [],
            "missing": [],
        })]
    );
    let driven = feature.drive(script).await.outcome;
    assert_eq!(driven.expect("drive returns"), DriveOutcome::Succeeded);
}

#[tokio::test(flavor = "multi_thread")]
async fn restoring_an_item_list_removes_new_files_recreates_missing_ones_and_rewrites_changed_ones()
{
    let feature = Feature::new(planned_catalog(TASK_WORKFLOW), "planned").await;
    let script = Script::new().on("plan", 1, 1, plan(&[("A", &[]), ("B", &[])]));
    let reached = feature.drive(script).await.outcome;
    assert!(
        matches!(reached, Ok(DriveOutcome::AtGate { .. })),
        "{reached:?}"
    );
    let plan_json = feature.read("plan.json");
    feature.edit("tasks/A.md", "# Task A\nmore\nand more\n");
    feature.edit("tasks/C.md", "# Task C\n");
    fs::remove_file(feature.working_copy("tasks/B.md")).expect("tasks/B.md is removed");
    let revisions = feature.revisions().await;

    let restored = feature.project.restore(SUBJECT, "plan").await;

    assert_eq!(
        restored.expect("the plan is restored").changes,
        [
            changed("tasks/A.md", 2, 0),
            file("tasks/B.md", Change::Missing),
            file("tasks/C.md", Change::New),
        ]
    );
    assert_eq!(feature.read("tasks/A.md").as_deref(), Some("# Task A\n"));
    assert_eq!(feature.read("tasks/B.md").as_deref(), Some("# Task B\n"));
    assert_eq!(feature.read("tasks/C.md"), None);
    assert_eq!(feature.read("plan.json"), plan_json);
    assert_eq!(feature.revisions().await, revisions);
    let baseline: Vec<(String, String)> = feature
        .dir
        .rows("SELECT rel_path, sha256 FROM working_copy_file ORDER BY rel_path")
        .await;
    let on_disk: Vec<(String, String)> = baseline
        .iter()
        .map(|(rel_path, _)| {
            let bytes = fs::read(feature.working_copy(rel_path)).expect("the file is back");
            (rel_path.clone(), crate::artifacts::sha256_hex(&bytes))
        })
        .collect();
    assert_eq!(baseline, on_disk);
}

#[tokio::test(flavor = "multi_thread")]
async fn restoring_an_unedited_artifact_changes_nothing_and_records_no_event() {
    let (feature, _) = pinned_and_edited().await;
    feature
        .project
        .restore(SUBJECT, "draft")
        .await
        .expect("the first restore");

    let again = feature.project.restore(SUBJECT, "draft").await;

    assert_eq!(again.expect("the second restore").changes, []);
    assert_eq!(
        feature.dir.event_data("working_copy.restored").await.len(),
        1
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn restore_settles_a_pending_working_copy_write_instead_of_taking_it_for_an_edit() {
    let feature = Feature::review().await;
    let first = Script::new().on("write", 1, 1, draft(DRAFT));
    feature
        .drive(first)
        .await
        .outcome
        .expect("the draft reaches the gate");
    feature
        .project
        .reject(&feature.run, None, "shorter", ACTOR)
        .await
        .expect("the draft is rejected");
    feature.project.crash_after(Checkpoint::ExecutionAccepted);
    let second = Script::new().on("write", 2, 1, draft("# Draft 2\n"));
    assert_crashed(
        feature.drive(second).await.outcome,
        Checkpoint::ExecutionAccepted,
    );
    let revisions = feature.revisions().await;

    let restored = feature.project.restore(SUBJECT, "draft").await;

    let restored = restored.expect("the restore settles the write");
    assert_eq!((restored.revision, restored.changes), (2, vec![]));
    assert_eq!(feature.read("draft.md").as_deref(), Some("# Draft 2\n"));
    assert_eq!(feature.revisions().await, revisions);
    let pending = feature
        .count("SELECT count(*) FROM working_copy_file WHERE pending_revision_id IS NOT NULL")
        .await;
    assert_eq!(pending, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn restore_refuses_while_a_driver_holds_the_run_naming_its_pid() {
    let (feature, _) = pinned_and_edited().await;
    let lock = feature
        .dir
        .path()
        .join(".surt/runtime/locks")
        .join(format!("{}.lock", feature.run.as_str()));
    let _held = platform::try_lock_exclusive(&lock)
        .expect("the lock opens")
        .expect("the lock is free");

    let restored = feature.project.restore(SUBJECT, "draft").await;

    let Refusal::DriverLive { subject, pid, .. } = refusal(restored) else {
        panic!("expected a live-driver refusal");
    };
    assert_eq!(subject, SUBJECT);
    assert_eq!(pid, Some(std::process::id()));
    assert_eq!(feature.read("draft.md").as_deref(), Some(EDITED));
}

#[tokio::test(flavor = "multi_thread")]
async fn restore_refuses_on_a_terminal_run_and_keeps_the_file() {
    let (feature, _) = pinned_and_edited().await;
    feature
        .project
        .cancel(&feature.run, tokio::sync::mpsc::unbounded_channel().0)
        .await
        .expect("the run is cancelled");

    let restored = feature.project.restore(SUBJECT, "draft").await;
    let impact = feature.project.restore_impact(SUBJECT, "draft").await;

    let expected = Refusal::TerminalRun {
        subject: SUBJECT.to_owned(),
        run: 1,
        files: vec!["draft.md".to_owned()],
    };
    assert_eq!(refusal(restored), expected);
    assert_eq!(refusal(impact), expected);
    assert_eq!(feature.read("draft.md").as_deref(), Some(EDITED));
}

#[tokio::test(flavor = "multi_thread")]
async fn restore_refuses_an_artifact_without_a_working_copy_and_lists_the_restorable_ones() {
    let (feature, _) = pinned_and_edited().await;

    let restored = feature.project.restore(SUBJECT, "summary").await;

    assert_eq!(
        refusal(restored),
        Refusal::NotRestorable {
            subject: SUBJECT.to_owned(),
            artifact: "summary".to_owned(),
            restorable: vec!["draft".to_owned()],
        }
    );
}
