use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use serde_json::json;

use super::super::fixtures::SUBJECT;
use super::fixtures::{ACTOR, Feature, agent, preserved};
use super::planned::{PLANNER, plan};
use super::repository::{commit_file, git};
use crate::project::{Catalog, Config, DriveOutcome};
use crate::runtime::{
    AgentRuntime, Behavior, Delivery, FakeRuntime, RuntimeCompletion, RuntimeError,
    RuntimeEventSink, RuntimeRequest, Script, StopSignal,
};

const COMMITTED_WORKFLOW: &str = r#"
schema = 1
name = "committed"

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

pub(super) const BUILD_WORKFLOW: &str = r#"
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

pub(super) const COMMITTER: &str = r#"
schema = 1
id = "builder"

[inputs.item]
format = "json"

[inputs.request]
format = "text"

[inputs.check-report]
format = "json"
optional = true

[outputs.commit]
format = "json"
checks = [{ kind = "commit_evidence" }]

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const CHECKED_REVIEWER: &str = r#"
schema = 1
id = "reviewer"
outcomes = ["approved", "changes_requested"]

[inputs.commit]
format = "json"

[inputs.check-report]
format = "json"

[outputs.feedback]
format = "text"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

pub(super) type Act = fn(&Path) -> String;

pub(super) struct Committing {
    fake: FakeRuntime,
    acts: BTreeMap<(u32, u32), Act>,
    requests: Mutex<Vec<RuntimeRequest>>,
}

impl Committing {
    pub(super) fn new(script: Script) -> Committing {
        Committing {
            fake: FakeRuntime::new(script),
            acts: BTreeMap::new(),
            requests: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn on(mut self, iteration: u32, attempt: u32, act: Act) -> Committing {
        self.acts.insert((iteration, attempt), act);
        self
    }

    pub(super) fn requests(&self) -> Vec<RuntimeRequest> {
        self.requests.lock().expect("the requests").clone()
    }
}

impl AgentRuntime for Committing {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        self.requests
            .lock()
            .expect("the requests")
            .push(preserved(&request));
        let step = &request.step;
        let act = step
            .step
            .ends_with("/build")
            .then(|| self.acts.get(&(step.iteration, step.attempt)))
            .flatten();
        let Some(act) = act else {
            return self.fake.execute(request, events, stop).await;
        };
        let sha = act(&request.launch.working_dir);
        let commit = json!({ "sha": sha }).to_string();
        let script = Script::new().on(
            &step.step,
            step.iteration,
            step.attempt,
            Behavior::Deliver(Delivery::new("done").output("commit", "commit.json", commit)),
        );
        FakeRuntime::new(script)
            .execute(request, events, stop)
            .await
    }
}

pub(super) fn committed_catalog(task: &'static str) -> Catalog {
    Catalog::new()
        .workflow("committed", COMMITTED_WORKFLOW)
        .workflow("task", task)
        .agent("planner", agent(PLANNER))
        .agent("builder", agent(COMMITTER))
        .agent("reviewer", agent(CHECKED_REVIEWER))
}

pub(super) async fn committed(task: &'static str, config: Config) -> Feature {
    let feature = Feature::on_repository(committed_catalog(task), "committed", config).await;
    let planned = feature
        .drive(Script::new().on("plan", 1, 1, plan(&[("A", &[])])))
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

pub(super) fn change(worktree: &Path) -> String {
    commit_file(worktree, "feature.txt", "the feature\n")
}

pub(super) async fn build_failures(feature: &Feature) -> Vec<(u32, String, String)> {
    feature
        .dir
        .rows(
            "SELECT e.attempt, e.error_code, e.error_message FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id \
             WHERE s.step_key = 'build' AND e.state = 'failed' ORDER BY e.attempt",
        )
        .await
}

async fn tip(feature: &Feature) -> String {
    let tips: Vec<(String,)> = feature
        .dir
        .rows(
            "SELECT b.tip_sha FROM run_branch b JOIN workflow_run w ON w.id = b.workflow_run_id \
             WHERE w.item_key = 'A'",
        )
        .await;
    tips[0].0.clone()
}

async fn refused_then_accepted(refused: Act) -> (Feature, Vec<(u32, String, String)>) {
    let feature = committed(BUILD_WORKFLOW, Config::default()).await;
    let runtime = Committing::new(Script::new())
        .on(1, 1, refused)
        .on(1, 2, change);

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let accepted = tip(&feature).await;
    assert_eq!(
        feature.git(&["log", "-1", "--format=%s", &accepted]),
        "feature.txt"
    );
    let failures = build_failures(&feature).await;
    (feature, failures)
}

fn refusal(failures: &[(u32, String, String)]) -> String {
    assert_eq!(failures.len(), 1, "{failures:?}");
    let (attempt, code, message) = &failures[0];
    assert_eq!((*attempt, code.as_str()), (1, "invalid_delivery"));
    message.clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_commit_on_the_task_branch_is_accepted_and_becomes_its_tip() {
    let feature = committed(BUILD_WORKFLOW, Config::default()).await;
    let runtime = Committing::new(Script::new()).on(1, 1, change);

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let committed = tip(&feature).await;
    assert_ne!(committed, feature.base());
    assert_eq!(
        feature.git(&["log", "-1", "--format=%s", &committed]),
        "feature.txt"
    );
    assert!(build_failures(&feature).await.is_empty());
}

fn unknown_sha(_: &Path) -> String {
    "0123456789abcdef0123456789abcdef01234567".to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_sha_shaped_string_that_names_no_commit_is_an_invalid_delivery() {
    let (_, failures) = refused_then_accepted(unknown_sha).await;

    assert_eq!(
        refusal(&failures),
        "commit: 0123456789abcdef0123456789abcdef01234567 is not a commit"
    );
}

fn detached(worktree: &Path) -> String {
    let sha = change(worktree);
    git(worktree, &["checkout", "--quiet", "--detach", &sha]);
    sha
}

#[tokio::test(flavor = "multi_thread")]
async fn a_commit_on_a_detached_head_is_an_invalid_delivery() {
    let (_, failures) = refused_then_accepted(detached).await;

    assert!(
        refusal(&failures).ends_with(&format!(
            " is not checked out as the tip of surt/task/{SUBJECT}/A"
        )),
        "{failures:?}"
    );
}

fn on_another_branch(worktree: &Path) -> String {
    git(worktree, &["checkout", "--quiet", "-b", "elsewhere"]);
    change(worktree)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_commit_on_another_branch_is_an_invalid_delivery() {
    let (_, failures) = refused_then_accepted(on_another_branch).await;

    assert!(
        refusal(&failures).contains(" is not checked out as the tip of "),
        "{failures:?}"
    );
}

fn unrelated_history(worktree: &Path) -> String {
    let tree = git(worktree, &["write-tree"]);
    let orphan = git(
        worktree,
        &["commit-tree", "--no-gpg-sign", &tree, "-m", "unrelated"],
    );
    git(worktree, &["reset", "--quiet", "--hard", &orphan]);
    orphan
}

#[tokio::test(flavor = "multi_thread")]
async fn a_commit_that_does_not_descend_from_the_task_start_is_an_invalid_delivery() {
    let (feature, failures) = refused_then_accepted(unrelated_history).await;

    assert!(
        refusal(&failures).ends_with(&format!(
            " does not descend from the child run's start {}",
            feature.base()
        )),
        "{failures:?}"
    );
}

fn unchanged(worktree: &Path) -> String {
    git(worktree, &["rev-parse", "HEAD"])
}

#[tokio::test(flavor = "multi_thread")]
async fn delivering_the_commit_the_iteration_started_at_is_an_invalid_delivery() {
    let (feature, failures) = refused_then_accepted(unchanged).await;

    assert_eq!(
        refusal(&failures),
        format!("commit: {} is not a new commit", feature.base())
    );
}

fn left_dirty(worktree: &Path) -> String {
    let sha = change(worktree);
    fs::write(worktree.join("stray.txt"), "uncommitted\n").expect("an untracked file");
    sha
}

#[tokio::test(flavor = "multi_thread")]
async fn a_commit_that_leaves_the_worktree_dirty_is_an_invalid_delivery() {
    let (_, failures) = refused_then_accepted(left_dirty).await;

    assert_eq!(
        refusal(&failures),
        "commit: the worktree has uncommitted changes"
    );
}

fn mid_cherry_pick(worktree: &Path) -> String {
    let sha = change(worktree);
    let marker = git(worktree, &["rev-parse", "--git-path", "CHERRY_PICK_HEAD"]);
    fs::write(worktree.join(marker), format!("{sha}\n")).expect("a cherry-pick marker");
    sha
}

#[tokio::test(flavor = "multi_thread")]
async fn a_commit_with_an_operation_in_progress_is_an_invalid_delivery() {
    let (_, failures) = refused_then_accepted(mid_cherry_pick).await;

    assert_eq!(
        refusal(&failures),
        "commit: a merge, rebase, cherry-pick or revert is in progress"
    );
}

fn nothing_to_change(worktree: &Path) -> String {
    git(
        worktree,
        &[
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "Nothing to change: README.md already says hello",
        ],
    );
    git(worktree, &["rev-parse", "HEAD"])
}

#[tokio::test(flavor = "multi_thread")]
async fn an_empty_commit_explaining_why_nothing_changed_is_a_valid_delivery() {
    let feature = committed(BUILD_WORKFLOW, Config::default()).await;
    let runtime = Committing::new(Script::new()).on(1, 1, nothing_to_change);

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert!(build_failures(&feature).await.is_empty());
    let accepted = tip(&feature).await;
    assert_eq!(
        feature.git(&["rev-parse", &format!("{accepted}^")]),
        feature.base()
    );
    assert_eq!(
        feature.git(&["diff", "--name-only", &feature.base(), &accepted]),
        ""
    );
}
