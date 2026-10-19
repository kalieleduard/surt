use std::fs;
use std::path::{Path, PathBuf};

use super::super::fixtures::SUBJECT;
use super::fixtures::{Driven, Feature};
use super::integration::{integration_branch, login, logout, task_branch_of};
use super::repository::git;
use super::team::{INTEGRATED_TASK, Team, Turn, builds, built, reviews, shipped, ships};
use crate::project::{Cleanup, Config, DriveOutcome, Kept, Progress};
use crate::runtime::{Behavior, Delivery};

pub(super) fn leaves_leftovers_and_crashes() -> Turn {
    Box::new(|worktree| {
        fs::write(worktree.join("scratch.txt"), "half done\n").expect("a leftover");
        Behavior::Crash { exit_code: 1 }
    })
}

pub(super) fn delivered_team() -> Team {
    Team::new()
        .on("A/build", 1, 1, builds(login))
        .on("A/review", 1, 1, reviews("approved"))
        .on("B/build", 1, 1, builds(logout))
        .on("B/review", 1, 1, reviews("approved"))
        .on("ship", 1, 1, ships())
}

fn shipping_after(meddle: impl Fn(&Path) + Send + Sync + 'static) -> Turn {
    Box::new(move |root| {
        meddle(root);
        let ship = ships();
        ship(root)
    })
}

pub(super) fn cleanups(driven: &Driven) -> Vec<Cleanup> {
    driven
        .progress
        .iter()
        .filter_map(|event| match event {
            Progress::CleanedUp(cleanup) => Some(cleanup.clone()),
            _ => None,
        })
        .collect()
}

impl Feature {
    pub(super) fn executions_dir(&self) -> PathBuf {
        self.root().join(".surt/runtime/executions")
    }

    pub(super) async fn cleaned_at(&self) -> Option<i64> {
        let rows: Vec<(Option<i64>,)> = self
            .dir
            .rows("SELECT cleaned_at FROM workflow_run WHERE parent_step_run_id IS NULL")
            .await;
        rows[0].0
    }

    pub(super) async fn events(&self, kind: &str) -> Vec<String> {
        let rows: Vec<(String,)> = self
            .dir
            .rows(&format!(
                "SELECT data FROM event WHERE kind = '{kind}' ORDER BY seq"
            ))
            .await;
        rows.into_iter().map(|(data,)| data).collect()
    }

    pub(super) fn surt_refs(&self) -> String {
        self.git(&["for-each-ref", "--format=%(refname)", "refs/surt/"])
    }

    fn worktree_list(&self) -> String {
        self.git(&["worktree", "list", "--porcelain"])
    }
}

pub(super) fn entries(dir: &Path) -> Vec<String> {
    match fs::read_dir(dir) {
        Ok(entries) => entries
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_succeeded_run_removes_its_scratch_space_and_keeps_its_product_and_history() {
    let feature = shipped(&[("A", &[]), ("B", &[])], Config::default()).await;
    let team = delivered_team()
        .on("A/build", 1, 1, leaves_leftovers_and_crashes())
        .on("A/build", 1, 2, builds(login));

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        driven.outcome.as_ref().expect("drive returns"),
        &DriveOutcome::Succeeded
    );
    assert_eq!(
        cleanups(&driven),
        [Cleanup {
            worktrees: 2,
            child_branches: 2,
            salvage_refs: 1,
            working_copy: true,
            kept: Vec::new(),
            left: Vec::new(),
        }]
    );
    for key in ["A", "B"] {
        assert!(!feature.worktree(key).exists());
        assert_eq!(feature.sha(&task_branch_of(key)), None);
    }
    assert!(!feature.worktree_list().contains(".surt/worktrees"));
    assert_eq!(feature.surt_refs(), "");
    assert!(feature.sha(&integration_branch()).is_some());
    assert!(!feature.working_copy("").exists());
    assert_eq!(entries(&feature.executions_dir()), Vec::<String>::new());
    assert!(!entries(&feature.root().join(".surt/logs").join(SUBJECT)).is_empty());
    assert!(feature.cleaned_at().await.is_some());
    assert_eq!(feature.events("cleanup.finished").await, [r#"{"kept":[]}"#]);
}

pub(super) fn leaves_leftovers_and_mismatches() -> Turn {
    Box::new(|worktree| {
        fs::write(worktree.join("half.txt"), "half\n").expect("a leftover");
        Behavior::ModelMismatch {
            reported: "another-model".to_owned(),
            delivery: Delivery::new("done"),
        }
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cancelled_run_salvages_and_removes_its_worktrees_but_keeps_its_refs() {
    let feature = built(
        INTEGRATED_TASK,
        &[("A", &[]), ("B", &[])],
        Config::default(),
    )
    .await;
    let team = delivered_team().on("B/build", 1, 1, leaves_leftovers_and_mismatches());
    let blocked = feature.drive_with(&team).await;
    assert!(matches!(
        blocked.outcome.expect("drive returns"),
        DriveOutcome::Blocked { .. }
    ));
    let recorded = feature.recorded_tips().await;
    feature
        .dir
        .execute(
            "UPDATE workflow_run SET cancel_requested_at = 1, \
             cancel_requested_by = 'dev@example.com' WHERE parent_step_run_id IS NULL",
        )
        .await;

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        driven.outcome.as_ref().expect("drive returns"),
        &DriveOutcome::Cancelled
    );
    assert_eq!(
        cleanups(&driven),
        [Cleanup {
            worktrees: 2,
            working_copy: true,
            ..Cleanup::default()
        }]
    );
    for (key, tip) in [("A", &recorded[1].1), ("B", &recorded[2].1)] {
        assert!(!feature.worktree(key).exists());
        assert_eq!(feature.sha(&task_branch_of(key)).as_ref(), Some(tip));
    }
    let last: Vec<(String,)> = feature
        .dir
        .rows(
            "SELECT e.id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id WHERE w.item_key = 'B' \
             ORDER BY e.started_at DESC, e.id DESC LIMIT 1",
        )
        .await;
    let salvage = format!("refs/surt/salvage/{}", last[0].0);
    assert_eq!(feature.surt_refs(), salvage);
    assert_eq!(
        feature.git(&["show", &format!("{salvage}:half.txt")]),
        "half"
    );
    assert!(feature.cleaned_at().await.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_working_copy_file_with_an_unrecorded_edit_is_kept_with_a_warning() {
    let feature = shipped(&[("A", &[]), ("B", &[])], Config::default()).await;
    let team = delivered_team().on(
        "ship",
        1,
        1,
        shipping_after(|root| {
            let task = root.join(".surt/subjects").join(SUBJECT).join("tasks/A.md");
            fs::write(task, "# Task A, edited by hand\n").expect("an edit");
        }),
    );

    let driven = feature.drive_with(&team).await;

    let kept = Path::new(".surt/subjects").join(SUBJECT).join("tasks/A.md");
    assert_eq!(
        cleanups(&driven)[0].kept,
        [Kept::UnrecordedEdit { path: kept.clone() }]
    );
    assert_eq!(
        fs::read_to_string(feature.root().join(&kept)).expect("the edit is kept"),
        "# Task A, edited by hand\n"
    );
    assert_eq!(entries(&feature.working_copy("")), ["tasks"]);
    assert_eq!(entries(&feature.working_copy("tasks")), ["A.md"]);
    assert!(feature.cleaned_at().await.is_some());
    let finished = feature.events("cleanup.finished").await;
    assert!(finished[0].contains("unrecorded_edit"), "{finished:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_moved_task_branch_and_a_foreign_path_are_kept_with_a_warning() {
    let feature = shipped(&[("A", &[]), ("B", &[])], Config::default()).await;
    let foreign = feature.worktree("B");
    let moved = foreign.clone();
    let team = delivered_team().on(
        "ship",
        1,
        1,
        shipping_after(move |root| {
            git(root, &["update-ref", &task_branch_of("A"), "main"]);
            git(
                root,
                &["worktree", "remove", "-f", "-f", &moved.to_string_lossy()],
            );
            fs::create_dir_all(&moved).expect("someone else's directory");
            fs::write(moved.join("mine.txt"), "mine\n").expect("someone else's file");
        }),
    );

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        cleanups(&driven),
        [Cleanup {
            worktrees: 1,
            child_branches: 1,
            working_copy: true,
            kept: vec![
                Kept::ForeignPath {
                    path: foreign.clone()
                },
                Kept::MovedBranch {
                    branch: format!("surt/item/{SUBJECT}/A")
                },
            ],
            ..Cleanup::default()
        }]
    );
    assert_eq!(feature.sha(&task_branch_of("A")), Some(feature.base()));
    assert_eq!(feature.sha(&task_branch_of("B")), None);
    assert_eq!(
        fs::read_to_string(foreign.join("mine.txt")).expect("the foreign file is kept"),
        "mine\n"
    );
    assert!(feature.cleaned_at().await.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_removal_failure_leaves_the_cleanup_pending_until_the_next_drive_finishes_it() {
    let feature = shipped(&[("A", &[]), ("B", &[])], Config::default()).await;
    let executions = feature.executions_dir();
    let blocker = executions.clone();
    let team = delivered_team().on(
        "ship",
        1,
        1,
        shipping_after(move |_| {
            let mut names = entries(&blocker);
            names.sort();
            let first = blocker.join(&names[0]);
            fs::remove_dir_all(&first).expect("the directory is replaced");
            fs::write(&first, "not a directory\n").expect("a file in its place");
        }),
    );

    let failed = feature.drive_with(&team).await;

    assert_eq!(
        failed.outcome.as_ref().expect("drive returns"),
        &DriveOutcome::Succeeded
    );
    let incomplete = cleanups(&failed);
    assert_eq!(incomplete[0].left.len(), 1, "{incomplete:?}");
    assert_eq!(incomplete[0].worktrees, 2);
    assert_eq!(feature.cleaned_at().await, None);
    assert_eq!(feature.events("cleanup.incomplete").await.len(), 1);
    let pending = feature
        .project
        .resolve_subject(SUBJECT)
        .await
        .expect("the feature");
    assert_eq!(
        (pending.active_run, pending.pending_cleanup),
        (None, Some(feature.run.clone()))
    );
    let mut names = entries(&executions);
    names.sort();
    fs::remove_file(executions.join(&names[0])).expect("the blocker is gone");

    let resumed = feature.drive_with(&team).await;

    assert_eq!(
        resumed.outcome.as_ref().expect("drive returns"),
        &DriveOutcome::Succeeded
    );
    assert_eq!(cleanups(&resumed), [Cleanup::default()]);
    assert!(feature.cleaned_at().await.is_some());
    assert_eq!(entries(&executions), Vec::<String>::new());
    let cleaned = feature
        .project
        .resolve_subject(SUBJECT)
        .await
        .expect("the feature");
    assert_eq!(cleaned.pending_cleanup, None);
    let again = feature.drive_with(&team).await;
    assert_eq!(
        again.outcome.as_ref().expect("drive returns"),
        &DriveOutcome::Succeeded
    );
    assert_eq!(cleanups(&again), []);
    assert_eq!(feature.events("cleanup.finished").await.len(), 1);
}
