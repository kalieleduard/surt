use std::fs;
use std::path::PathBuf;

use super::super::fixtures::{SUBJECT, input};
use super::fixtures::{ACTOR, Feature, draft, review_catalog};
use super::integration::{login, logout};
use super::planned::plan;
use super::team::{INTEGRATED_TASK, Team, Turn, builds, reviews, shipped, ships, team_catalog};
use super::worktrees::refused;
use crate::platform;
use crate::project::{
    BranchFate, Carry, Config, DeletedRun, Deletion, DriveOutcome, Error, NewRun, Refusal, RunState,
};
use crate::runtime::Script;

const ITEMS: &[(&str, &[&str])] = &[("A", &[]), ("B", &[])];

fn delivered_team() -> Team {
    Team::new()
        .on("A/build", 1, 1, builds(login))
        .on("A/review", 1, 1, reviews("approved"))
        .on("B/build", 1, 1, builds(logout))
        .on("B/review", 1, 1, reviews("approved"))
        .on("ship", 1, 1, ships())
}

fn leaves_leftovers_and_is_refused() -> Turn {
    Box::new(|worktree| {
        fs::write(worktree.join("half.txt"), "half\n").expect("a leftover");
        refused()
    })
}

fn next_run(subject: &str) -> NewRun {
    NewRun {
        subject: subject.to_owned(),
        workflow: "shipped".to_owned(),
        inputs: vec![input("request", "add login"), input("base", "main")],
        carry: Carry::Nothing,
        merge_into_base: None,
        actor: ACTOR.to_owned(),
    }
}

async fn cancelled() -> Feature {
    let feature = shipped(ITEMS, Config::default()).await;
    let team = delivered_team().on("B/build", 1, 1, leaves_leftovers_and_is_refused());
    let blocked = feature.drive_with(&team).await.outcome;
    assert!(
        matches!(blocked, Ok(DriveOutcome::Blocked { .. })),
        "{blocked:?}"
    );
    feature.cancel().await.outcome.expect("run 1 is cancelled");
    feature
}

async fn cancelled_then_delivered() -> Feature {
    let mut feature = cancelled().await;
    feature.run = feature
        .project
        .create_run(next_run(SUBJECT))
        .await
        .expect("run 2 is created")
        .run;
    feature
        .drive(Script::new().on("plan", 1, 1, plan(ITEMS)))
        .await
        .outcome
        .expect("the plan reaches its gate");
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the plan is approved");
    let delivered = feature.drive_with(&delivered_team()).await.outcome;
    assert!(
        matches!(delivered, Ok(DriveOutcome::Succeeded)),
        "{delivered:?}"
    );
    feature
}

impl Feature {
    fn surt_ref_names(&self) -> Vec<String> {
        self.git(&[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads/surt/",
            "refs/surt/",
        ])
        .lines()
        .map(str::to_owned)
        .collect()
    }

    fn surt_dir(&self, path: &str) -> PathBuf {
        self.root().join(".surt").join(path)
    }

    async fn scalar(&self, sql: &str) -> i64 {
        let rows: Vec<(i64,)> = self.dir.rows(sql).await;
        rows[0].0
    }

    async fn roots(&self) -> i64 {
        self.scalar("SELECT count(*) FROM workflow_run WHERE parent_step_run_id IS NULL")
            .await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_keeps_only_the_delivered_integration_branch_and_purges_the_history() {
    let feature = cancelled_then_delivered().await;
    let other = "20261017-other";
    feature
        .project
        .create_run(next_run(other))
        .await
        .expect("another feature");
    let run_locks: Vec<(String,)> = feature
        .dir
        .rows(&format!(
            "SELECT w.id FROM workflow_run w JOIN subject s ON s.id = w.subject_id \
             WHERE w.parent_step_run_id IS NULL AND s.name = '{SUBJECT}'"
        ))
        .await;
    assert_eq!(run_locks.len(), 2);
    let locks = feature.surt_dir("runtime/locks");
    assert!(
        locks
            .join(format!("{}.lock", feature.run.as_str()))
            .exists()
    );
    let mine = feature.dir.home().join("mine");
    feature.git(&[
        "worktree",
        "add",
        "--quiet",
        "-b",
        "mine",
        &mine.to_string_lossy(),
    ]);

    let deleted = feature.project.delete(SUBJECT).await;

    assert_eq!(deleted.expect("the feature is deleted"), Deletion::Deleted);
    assert_eq!(
        feature.surt_ref_names(),
        [format!("refs/heads/surt/{SUBJECT}.2")]
    );
    let listed = feature.git(&["worktree", "list"]);
    assert!(!listed.contains(".surt/worktrees"), "{listed}");
    assert!(listed.contains("[mine]"), "{listed}");
    assert!(feature.sha("refs/heads/main").is_some());
    for gone in [format!("logs/{SUBJECT}"), format!("features/{SUBJECT}")] {
        assert!(!feature.surt_dir(&gone).exists(), "{gone}");
    }
    assert_eq!(
        fs::read_dir(feature.surt_dir("runtime/executions"))
            .map(Iterator::count)
            .unwrap_or_default(),
        0
    );
    let subjects: Vec<(String,)> = feature.dir.rows("SELECT name FROM subject").await;
    assert_eq!(subjects, [(other.to_owned(),)]);
    assert_eq!(feature.roots().await, 1);
    for unreferenced in [
        "SELECT count(*) FROM blob WHERE sha256 NOT IN (SELECT blob_sha256 FROM revision_file) \
         AND sha256 NOT IN (SELECT blob_sha256 FROM snapshot_file)",
        "SELECT count(*) FROM snapshot WHERE sha256 NOT IN (SELECT snapshot_sha256 FROM run_snapshot) \
         AND sha256 NOT IN (SELECT definition_sha256 FROM workflow_run)",
        "SELECT count(*) FROM event WHERE root_run_id NOT IN (SELECT id FROM workflow_run)",
    ] {
        assert_eq!(feature.scalar(unreferenced).await, 0, "{unreferenced}");
    }
    assert!(feature.scalar("SELECT count(*) FROM blob").await > 0);
    for (run,) in run_locks {
        assert!(!locks.join(format!("{run}.lock")).exists(), "{run}");
    }
    let mut slots: Vec<String> = fs::read_dir(feature.surt_dir("runtime/slots"))
        .expect("the project's agent slots are kept")
        .map(|entry| {
            entry
                .expect("a slot")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    slots.sort();
    assert_eq!(slots, ["agent-0.lock", "agent-1.lock", "agent-2.lock"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_an_unknown_feature_is_refused() {
    let feature = cancelled().await;

    let deleted = feature.project.delete("20200101-nothing").await;

    assert!(
        matches!(
            &deleted,
            Err(Error::Refused(Refusal::UnknownSubject { .. }))
        ),
        "{deleted:?}"
    );
    assert_eq!(feature.roots().await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_delete_impact_lists_runs_refs_directories_edits_and_history() {
    let feature = cancelled_then_delivered().await;
    fs::create_dir_all(feature.working_copy("")).expect("the working copy");
    fs::write(feature.working_copy("notes.md"), "mine\n").expect("a stray file");

    let impact = feature
        .project
        .delete_impact(SUBJECT)
        .await
        .expect("the impact");

    assert_eq!(
        impact.runs,
        [
            DeletedRun {
                run: 1,
                state: RunState::Cancelled
            },
            DeletedRun {
                run: 2,
                state: RunState::Succeeded
            },
        ]
    );
    assert_eq!(
        impact.integration_branches,
        [
            BranchFate {
                branch: format!("surt/{SUBJECT}"),
                run: 1,
                kept: false
            },
            BranchFate {
                branch: format!("surt/{SUBJECT}.2"),
                run: 2,
                kept: true
            },
        ]
    );
    assert_eq!(
        impact.item_branches,
        [
            format!("surt/task/{SUBJECT}/A"),
            format!("surt/task/{SUBJECT}/B")
        ]
    );
    assert_eq!(impact.salvage_refs, 1);
    let logs = PathBuf::from(".surt/logs").join(SUBJECT);
    assert_eq!(impact.directories[0].path, logs);
    assert!(impact.directories[0].bytes > 0);
    assert_eq!(
        impact.directories[1].path,
        PathBuf::from(".surt/features").join(SUBJECT)
    );
    assert_eq!(impact.directories.len(), 2);
    assert_eq!(impact.unrecorded_edits, Vec::<PathBuf>::new());
    assert_eq!(
        impact.executions,
        usize::try_from(feature.scalar("SELECT count(*) FROM execution").await).expect("a count")
    );
    assert_eq!(
        impact.events,
        usize::try_from(feature.scalar("SELECT count(*) FROM event").await).expect("a count")
    );
    assert!(impact.revisions > 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_delete_impact_counts_unrecorded_edits_in_the_working_copy() {
    let feature = Feature::on_repository(review_catalog(), "review", Config::default()).await;
    feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await
        .outcome
        .expect("the draft reaches the gate");
    feature.edit("draft.md", "# Draft 1, edited by hand\n");
    feature
        .cancel()
        .await
        .outcome
        .expect("the run is cancelled");

    let impact = feature
        .project
        .delete_impact(SUBJECT)
        .await
        .expect("the impact");

    assert_eq!(
        impact.unrecorded_edits,
        [PathBuf::from(".surt/features")
            .join(SUBJECT)
            .join("draft.md")]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_active_run_refuses_the_delete_and_its_impact() {
    let feature = Feature::review().await;

    let impact = feature.project.delete_impact(SUBJECT).await;
    let deleted = feature.project.delete(SUBJECT).await;

    for refused in [impact.map(|_| ()), deleted.map(|_| ())] {
        assert!(
            matches!(&refused, Err(Error::Refused(Refusal::ActiveRunBlocksDelete { subject })) if subject == SUBJECT),
            "{refused:?}"
        );
    }
    assert_eq!(feature.roots().await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_busy_run_lock_refuses_the_delete_and_changes_nothing() {
    let feature = cancelled().await;
    let refs = feature.surt_ref_names();
    let lock = feature
        .surt_dir("runtime/locks")
        .join(format!("{}.lock", feature.run.as_str()));
    let _held = platform::try_lock_exclusive(&lock)
        .expect("the lock file opens")
        .expect("the lock is free");

    let deleted = feature.project.delete(SUBJECT).await;

    assert!(
        matches!(&deleted, Err(Error::Refused(Refusal::RunLockBusy { subject })) if subject == SUBJECT),
        "{deleted:?}"
    );
    assert_eq!(feature.surt_ref_names(), refs);
    assert!(feature.surt_dir(&format!("logs/{SUBJECT}")).is_dir());
    assert_eq!(feature.roots().await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_removal_failure_leaves_the_rows_and_a_rerun_finishes_the_delete() {
    let feature = cancelled().await;
    let logs = feature.surt_dir(&format!("logs/{SUBJECT}"));
    fs::remove_dir_all(&logs).expect("the logs are replaced");
    fs::write(&logs, "not a directory\n").expect("a file in their place");

    let failed = feature.project.delete(SUBJECT).await;

    match failed.expect("the delete returns") {
        Deletion::Incomplete { left } => {
            assert_eq!(left.len(), 1, "{left:?}");
            assert_eq!(left[0].item, format!(".surt/logs/{SUBJECT}"));
        }
        Deletion::Deleted => panic!("the logs could not be removed"),
    }
    assert_eq!(feature.surt_ref_names(), Vec::<String>::new());
    assert!(!feature.working_copy("").exists());
    assert_eq!(feature.roots().await, 1);
    let lock = feature
        .surt_dir("runtime/locks")
        .join(format!("{}.lock", feature.run.as_str()));
    assert!(lock.exists());
    fs::remove_file(&logs).expect("the blocker is gone");

    let rerun = feature.project.delete(SUBJECT).await;

    assert_eq!(rerun.expect("the rerun"), Deletion::Deleted);
    assert!(!logs.exists());
    assert_eq!(feature.scalar("SELECT count(*) FROM subject").await, 0);
    assert!(!lock.exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_run_created_while_deleting_rolls_the_purge_back() {
    let feature = cancelled().await;
    let other = feature
        .dir
        .project(team_catalog(INTEGRATED_TASK), Config::default())
        .await;
    let next = next_run(SUBJECT);
    feature.project.interleave_before_purge(async move {
        other.create_run(next).await.expect("run 2 is created");
    });

    let deleted = feature.project.delete(SUBJECT).await;

    assert!(
        matches!(&deleted, Err(Error::Refused(Refusal::SubjectChangedWhileDeleting { subject })) if subject == SUBJECT),
        "{deleted:?}"
    );
    assert_eq!(feature.roots().await, 2);
    assert!(feature.scalar("SELECT count(*) FROM event").await > 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn worktrees_left_by_an_unfinished_cleanup_are_removed_and_a_missing_one_is_pruned() {
    let feature = shipped(ITEMS, Config::default()).await;
    let team = delivered_team().on("B/build", 1, 1, Box::new(|_| refused()));
    let blocked = feature.drive_with(&team).await.outcome;
    assert!(
        matches!(blocked, Ok(DriveOutcome::Blocked { .. })),
        "{blocked:?}"
    );
    feature
        .dir
        .execute(
            "UPDATE workflow_run SET state = 'cancelled', finished_at = 1 \
             WHERE parent_step_run_id IS NULL",
        )
        .await;
    let (kept, missing) = (feature.worktree("A"), feature.worktree("B"));
    assert!(kept.is_dir() && missing.is_dir());
    fs::remove_dir_all(&missing).expect("the worktree directory is gone");

    let deleted = feature.project.delete(SUBJECT).await;

    assert_eq!(deleted.expect("the feature is deleted"), Deletion::Deleted);
    assert!(!kept.exists());
    let listed = feature.git(&["worktree", "list", "--porcelain"]);
    assert!(!listed.contains(".surt/worktrees"), "{listed}");
    assert_eq!(feature.surt_ref_names(), Vec::<String>::new());
}
