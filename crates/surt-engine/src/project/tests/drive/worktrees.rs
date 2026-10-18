use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::super::fixtures::SUBJECT;
use super::fixtures::{ACTOR, Feature, Recording, agent};
use super::planned::{BUILDER, PLANNER, plan, work};
use super::repository::{commit_file, git, git_succeeds};
use crate::project::{Catalog, Config, DriveOutcome, Progress, Refusal, StepLabel};
use crate::runtime::{
    AgentRuntime, Behavior, FakeRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink,
    RuntimeRequest, Script, StopSignal,
};

const TRACKED_WORKFLOW: &str = r#"
schema = 1
name = "tracked"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.plan]
kind = "agent"
agent = "planner"
needs = ["prepare"]

[steps.approve]
kind = "approval"
artifact = "plan"
rejection_artifact = "plan-rejection"
needs = ["plan"]

[steps.implement]
kind = "per_item"
items = "plan"
workflow = "task"
pass = ["request"]
needs = ["approve"]
"#;

const TASK_WORKFLOW: &str = r#"
schema = 1
name = "task"

[steps.setup]
kind = "operation"
run = "git.create_child_worktree"

[steps.build]
kind = "agent"
agent = "builder"
needs = ["setup"]
"#;

fn tracked_catalog() -> Catalog {
    Catalog::new()
        .workflow("tracked", TRACKED_WORKFLOW)
        .workflow("task", TASK_WORKFLOW)
        .agent("planner", agent(PLANNER))
        .agent("builder", agent(BUILDER))
}

async fn tracked(items: &[(&str, &[&str])], config: Config) -> Feature {
    let feature = Feature::on_repository(tracked_catalog(), "tracked", config).await;
    let planned = feature
        .drive(Script::new().on("plan", 1, 1, plan(items)))
        .await
        .outcome
        .expect("the plan reaches its gate");
    assert!(
        matches!(planned, DriveOutcome::AtGate { .. }),
        "{planned:?}"
    );
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the plan is approved");
    feature
}

fn task_branch(key: &str) -> String {
    format!("refs/heads/surt/task/{SUBJECT}/{key}")
}

async fn child_run(feature: &Feature, key: &'static str) -> String {
    let runs: Vec<(String, Option<String>)> = feature
        .dir
        .rows("SELECT id, item_key FROM workflow_run")
        .await;
    runs.into_iter()
        .find(|(_, item)| item.as_deref() == Some(key))
        .map(|(id, _)| id)
        .expect("the child run exists")
}

#[tokio::test(flavor = "multi_thread")]
async fn setup_creates_a_locked_worktree_on_a_task_branch_at_the_integration_tip() {
    let feature = tracked(&[("A", &[])], Config::default()).await;
    let base = feature.base();
    let runtime = Recording::new(Script::new().on("A/build", 1, 1, work("a")));

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let worktree = feature.worktree("A");
    let recorded: Vec<(String, String, String, Option<String>)> = feature
        .dir
        .rows(
            "SELECT b.ref_name, b.start_sha, b.tip_sha, b.worktree_path FROM run_branch b \
             JOIN workflow_run w ON w.id = b.workflow_run_id WHERE w.item_key = 'A'",
        )
        .await;
    assert_eq!(
        recorded,
        [(
            task_branch("A"),
            base.clone(),
            base.clone(),
            Some(worktree.display().to_string())
        )]
    );
    assert_eq!(feature.sha(&task_branch("A")), Some(base));
    let listed = feature.git(&["worktree", "list", "--porcelain"]);
    let entry = format!(
        "worktree {}\nHEAD {}\nbranch {}\nlocked surt {}",
        worktree.display(),
        feature.base(),
        task_branch("A"),
        child_run(&feature, "A").await
    );
    assert!(listed.contains(&entry), "{listed}");
    let requests = runtime.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].launch.working_dir, worktree);
}

struct Meddling {
    fake: FakeRuntime,
    meddle: fn(&Path),
    seen: Mutex<Vec<Seen>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Seen {
    attempt: u32,
    head: String,
    status: String,
    files: Vec<&'static str>,
}

const WATCHED: [&str; 4] = ["README.md", "scratch.txt", "wip.txt", "cache/warm.bin"];

impl Meddling {
    fn new(script: Script, meddle: fn(&Path)) -> Meddling {
        Meddling {
            fake: FakeRuntime::new(script),
            meddle,
            seen: Mutex::new(Vec::new()),
        }
    }

    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().expect("the observations").clone()
    }
}

impl AgentRuntime for Meddling {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let dir = request.launch.working_dir.clone();
        let seen = Seen {
            attempt: request.step.attempt,
            head: git(&dir, &["rev-parse", "HEAD"]),
            status: git(&dir, &["status", "--porcelain", "--untracked-files=all"]),
            files: WATCHED
                .into_iter()
                .filter(|file| dir.join(file).exists())
                .collect(),
        };
        self.seen.lock().expect("the observations").push(seen);
        if request.step.attempt == 1 {
            (self.meddle)(&dir);
        }
        self.fake.execute(request, events, stop).await
    }
}

fn leave_leftovers(worktree: &Path) {
    commit_file(worktree, "wip.txt", "unaccepted\n");
    fs::write(worktree.join("README.md"), "half edited\n").expect("an edit");
    fs::write(worktree.join("scratch.txt"), "scratch\n").expect("an untracked file");
    fs::create_dir_all(worktree.join("cache")).expect("a cache directory");
    fs::write(worktree.join("cache/warm.bin"), "warm\n").expect("an ignored file");
}

#[tokio::test(flavor = "multi_thread")]
async fn leftovers_of_a_failed_attempt_are_salvaged_and_the_next_attempt_starts_clean() {
    let feature = tracked(&[("A", &[])], Config::default()).await;
    fs::write(feature.root().join(".git/info/exclude"), ".surt/\ncache/\n")
        .expect("the exclude file");
    let base = feature.base();
    let runtime = Meddling::new(
        Script::new()
            .on("A/build", 1, 1, Behavior::Crash { exit_code: 1 })
            .on("A/build", 1, 2, work("a")),
        leave_leftovers,
    );

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let seen = runtime.seen();
    assert_eq!(
        seen[1],
        Seen {
            attempt: 2,
            head: base.clone(),
            status: String::new(),
            files: vec!["README.md", "cache/warm.bin"],
        }
    );
    let failed: Vec<(String,)> = feature
        .dir
        .rows("SELECT id FROM execution WHERE state = 'failed'")
        .await;
    let salvage = format!("refs/surt/salvage/{}", failed[0].0);
    let wip = feature.git(&["rev-parse", &format!("{salvage}^1")]);
    assert_eq!(feature.git(&["log", "--format=%s", "-1", &wip]), "wip.txt");
    assert_eq!(
        feature.git(&["show", &format!("{salvage}:README.md")]),
        "half edited"
    );
    assert_eq!(
        feature.git(&["show", &format!("{salvage}:scratch.txt")]),
        "scratch"
    );
    assert!(!git_succeeds(
        &feature.root(),
        &["cat-file", "-e", &format!("{salvage}:cache/warm.bin")]
    ));
    assert_eq!(feature.sha(&task_branch("A")), Some(base));
    let events: Vec<(String,)> = feature
        .dir
        .rows("SELECT data FROM event WHERE kind = 'worktree.salvaged'")
        .await;
    assert_eq!(events, [(format!(r#"{{"ref":"{salvage}"}}"#),)]);
    assert!(driven.progress.contains(&Progress::Salvaged {
        step: StepLabel {
            key: "build".to_owned(),
            item: Some("A".to_owned()),
            iteration: None,
        },
        reference: salvage,
    }));
}

fn admin_dir(worktree: &Path) -> PathBuf {
    let link = fs::read_to_string(worktree.join(".git")).expect("a linked worktree");
    PathBuf::from(link.trim().trim_start_matches("gitdir:").trim())
}

fn half_create(worktree: &Path) {
    let admin = admin_dir(worktree);
    fs::remove_file(admin.join("index")).expect("the index is removed");
    fs::write(admin.join("index.lock"), "").expect("a stale index lock");
    fs::remove_file(worktree.join("README.md")).expect("a partial checkout");
}

fn delete_directory(worktree: &Path) {
    fs::remove_dir_all(worktree).expect("the worktree is deleted");
}

fn delete_metadata(worktree: &Path) {
    fs::write(worktree.join("scratch.txt"), "unsaved\n").expect("an unsaved file");
    fs::remove_dir_all(admin_dir(worktree)).expect("the metadata is deleted");
}

fn conflict_merge(worktree: &Path) {
    git(worktree, &["checkout", "--quiet", "-b", "side"]);
    commit_file(worktree, "README.md", "side\n");
    git(worktree, &["checkout", "--quiet", "-"]);
    commit_file(worktree, "README.md", "ours\n");
    assert!(!git_succeeds(worktree, &["merge", "--quiet", "side"]));
}

async fn rebuilt_after(meddle: fn(&Path)) -> (Feature, Meddling, Vec<Progress>) {
    let feature = tracked(&[("A", &[])], Config::default()).await;
    let runtime = Meddling::new(
        Script::new()
            .on("A/build", 1, 1, Behavior::Crash { exit_code: 1 })
            .on("A/build", 1, 2, work("a")),
        meddle,
    );
    let driven = feature.drive_with(&runtime).await;
    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    (feature, runtime, driven.progress)
}

fn clean_start(feature: &Feature, runtime: &Meddling) {
    assert_eq!(
        runtime.seen()[1],
        Seen {
            attempt: 2,
            head: feature.base(),
            status: String::new(),
            files: vec!["README.md"],
        }
    );
}

fn repaired(feature: &Feature, progress: &[Progress]) -> bool {
    progress.contains(&Progress::WorktreeRepaired {
        step: StepLabel {
            key: "build".to_owned(),
            item: Some("A".to_owned()),
            iteration: None,
        },
        path: feature.worktree("A"),
    })
}

async fn repairs(feature: &Feature) -> Vec<String> {
    let events: Vec<(String,)> = feature
        .dir
        .rows("SELECT data ->> '$.repair' FROM event WHERE kind = 'worktree.repaired'")
        .await;
    events.into_iter().map(|(repair,)| repair).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_half_created_worktree_is_removed_and_added_again() {
    let (feature, runtime, progress) = rebuilt_after(half_create).await;

    clean_start(&feature, &runtime);
    assert!(repaired(&feature, &progress), "{progress:?}");
    assert_eq!(repairs(&feature).await, ["half_created"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_deleted_worktree_directory_is_recreated_at_its_recorded_path() {
    let (feature, runtime, progress) = rebuilt_after(delete_directory).await;

    clean_start(&feature, &runtime);
    assert!(repaired(&feature, &progress), "{progress:?}");
    assert_eq!(repairs(&feature).await, ["missing"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_orphaned_worktree_directory_is_moved_aside_and_the_worktree_added_again() {
    let (feature, runtime, progress) = rebuilt_after(delete_metadata).await;

    clean_start(&feature, &runtime);
    assert!(repaired(&feature, &progress), "{progress:?}");
    assert_eq!(repairs(&feature).await, ["orphaned"]);
    let worktree = feature.worktree("A");
    let parent = worktree.parent().expect("a parent directory");
    let aside: Vec<PathBuf> = fs::read_dir(parent)
        .expect("the subject directory")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| *path != worktree)
        .collect();
    assert_eq!(aside.len(), 1, "{aside:?}");
    assert_eq!(
        fs::read_to_string(aside[0].join("scratch.txt")).expect("the unsaved file survives"),
        "unsaved\n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_merge_left_in_progress_is_salvaged_and_cleared() {
    let (feature, runtime, progress) = rebuilt_after(conflict_merge).await;

    clean_start(&feature, &runtime);
    let salvaged: Vec<(String,)> = feature
        .dir
        .rows("SELECT data ->> '$.ref' FROM event WHERE kind = 'worktree.salvaged'")
        .await;
    let conflicted = feature.git(&["show", &format!("{}:README.md", salvaged[0].0)]);
    assert!(conflicted.contains("<<<<<<<"), "{conflicted}");
    assert!(
        progress
            .iter()
            .any(|event| matches!(event, Progress::Salvaged { .. }))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_path_surt_did_not_create_blocks_setup_and_is_left_untouched() {
    let feature = tracked(&[("A", &[])], Config::default()).await;
    let worktree = feature.worktree("A");
    fs::create_dir_all(&worktree).expect("someone else's directory");
    fs::write(worktree.join("mine.txt"), "mine\n").expect("someone else's file");

    let driven = feature.drive(Script::new()).await;

    assert!(matches!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Blocked { .. }
    ));
    let failures: Vec<(String, String, String)> = feature
        .dir
        .rows(
            "SELECT e.failure_class, e.error_code, e.error_message FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id WHERE s.step_key = 'setup'",
        )
        .await;
    assert_eq!(
        failures,
        [(
            "not_retryable".to_owned(),
            "worktree_path_foreign".to_owned(),
            format!(
                "{} exists but is not a worktree surt created",
                worktree.display()
            )
        )]
    );
    assert_eq!(
        fs::read_to_string(worktree.join("mine.txt")).expect("the file is untouched"),
        "mine\n"
    );
}

async fn setup_failures(feature: &Feature) -> Vec<(String, String, String)> {
    feature
        .dir
        .rows(
            "SELECT e.failure_class, e.error_code, e.error_message FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id WHERE s.step_key = 'setup'",
        )
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_driving_command_refuses_while_the_integration_branch_moved_and_resumes_once_restored() {
    let feature = tracked(&[("A", &[])], Config::default()).await;
    let base = feature.base();
    let moved = commit_file(&feature.root(), "moved.txt", "moved\n");
    let branch = format!("surt/{SUBJECT}");
    feature.git(&["branch", "--force", &branch, &moved]);

    let refused = feature.drive(Script::new()).await;

    assert_eq!(
        refused.outcome.expect("drive returns"),
        DriveOutcome::Halted(Refusal::IntegrationBranchMoved {
            subject: SUBJECT.to_owned(),
            branch: branch.clone(),
            expected: base.clone(),
            found: Some(moved),
        })
    );
    assert!(setup_failures(&feature).await.is_empty());
    assert!(!feature.worktree("A").exists());

    feature.git(&["branch", "--force", &branch, &base]);
    let restored = feature
        .drive(Script::new().on("A/build", 1, 1, work("a")))
        .await;

    assert_eq!(
        restored.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_driving_command_refuses_while_the_integration_branch_is_checked_out() {
    let feature = tracked(&[("A", &[])], Config::default()).await;
    let branch = format!("surt/{SUBJECT}");
    feature.git(&["checkout", "--quiet", &branch]);

    let driven = feature.drive(Script::new()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Halted(Refusal::IntegrationBranchCheckedOut {
            subject: SUBJECT.to_owned(),
            branch,
            path: feature.root(),
        })
    );
    assert!(setup_failures(&feature).await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn parallel_tasks_each_get_their_own_worktree_from_the_same_integration_tip() {
    let feature = tracked(&[("A", &[]), ("B", &[])], Config::default()).await;
    let base = feature.base();
    let script = Script::new()
        .on("A/build", 1, 1, work("a"))
        .on("B/build", 1, 1, work("b"));

    let driven = feature.drive(script).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let starts: Vec<(String, String)> = feature
        .dir
        .rows(
            "SELECT ref_name, start_sha FROM run_branch WHERE worktree_path IS NOT NULL \
             ORDER BY ref_name",
        )
        .await;
    assert_eq!(
        starts,
        [
            (task_branch("A"), base.clone()),
            (task_branch("B"), base.clone())
        ]
    );
    for key in ["A", "B"] {
        assert_eq!(
            git(&feature.worktree(key), &["symbolic-ref", "HEAD"]),
            task_branch(key)
        );
    }
}
