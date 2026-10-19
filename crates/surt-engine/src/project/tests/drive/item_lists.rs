use std::fs;

use super::super::fixtures::Catalog;
use super::super::fixtures::SUBJECT;
use super::super::refusal;
use super::fixtures::{ACTOR, Feature, agent};
use super::pinning::Editing;
use super::planned::{TASK_WORKFLOW, approved, work};
use crate::project::{DriveOutcome, Refusal, RevisionOrigin};
use crate::runtime::{Behavior, Delivery, Script};

const PLANNING_WORKFLOW: &str = r#"
schema = 1
name = "planning"

[steps.plan]
kind = "agent"
agent = "planner"

[steps.approve]
kind = "approval"
artifact = "plan"
rejection_artifact = "plan-rejection"
needs = ["plan"]

[steps.planning]
kind = "loop"
body = ["plan", "approve"]
until = { step = "approve", outcome = "approved" }

[steps.build]
kind = "agent"
agent = "builder"
needs = ["planning"]
"#;

const PLANNER: &str = r#"
schema = 1
id = "planner"

[inputs.request]
format = "text"

[outputs.plan]
format = "item_list"

[outputs.plan.item_files.task]
format = "markdown"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const BUILDER: &str = r#"
schema = 1
id = "builder"

[inputs.plan]
format = "item_list"

[outputs.report]
format = "text"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

fn item(key: &str, needs: &[&str], task: &str) -> String {
    serde_json::json!({ "key": key, "needs": needs, "files": { "task": task } }).to_string()
}

fn list(items: &[String]) -> String {
    format!("[{}]", items.join(", "))
}

fn plan(keys: &[&str]) -> Behavior {
    let items: Vec<String> = keys
        .iter()
        .map(|key| item(key, &[], &format!("tasks/{key}.md")))
        .collect();
    let delivery = keys.iter().fold(
        Delivery::new("done").output("plan", "plan.json", list(&items)),
        |delivery, key| delivery.file(&format!("tasks/{key}.md"), format!("# {key}\n")),
    );
    Behavior::Deliver(delivery)
}

fn planning_catalog() -> Catalog {
    Catalog::new()
        .workflow("planning", PLANNING_WORKFLOW)
        .agent("planner", agent(PLANNER))
        .agent("builder", agent(BUILDER))
}

async fn plan_at_gate() -> Feature {
    let feature = Feature::new(planning_catalog(), "planning").await;
    feature
        .drive(Script::new().on("plan", 1, 1, plan(&["A", "B"])))
        .await
        .outcome
        .expect("the plan reaches the gate");
    feature
}

async fn plan_revisions(feature: &Feature) -> Vec<(i64, i64)> {
    feature
        .dir
        .rows(
            "SELECT r.number, count(f.rel_path) FROM artifact_revision r \
             JOIN artifact a ON a.id = r.artifact_id JOIN revision_file f ON f.revision_id = r.id \
             WHERE a.name = 'plan' GROUP BY r.id ORDER BY r.number",
        )
        .await
}

fn invalid(problem: &str, files: &[&str]) -> Refusal {
    Refusal::InvalidEdit {
        subject: SUBJECT.to_owned(),
        artifact: "plan".to_owned(),
        problem: problem.to_owned(),
        files: files.iter().map(|file| (*file).to_owned()).collect(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_index_and_every_task_file_are_one_revision_so_editing_a_task_needs_reapproval() {
    let feature = Feature::new(planning_catalog(), "planning").await;
    feature
        .drive(Script::new().on("plan", 1, 1, plan(&["A", "B"])))
        .await
        .outcome
        .expect("the plan reaches the gate");
    feature.edit("tasks/A.md", "# A, narrowed by hand\n");

    let approved = feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the edit is approved");

    assert_eq!(
        (approved.revision, approved.origin, approved.files),
        (
            2,
            RevisionOrigin::Human,
            vec![
                "plan.json".to_owned(),
                "tasks/A.md".to_owned(),
                "tasks/B.md".to_owned()
            ]
        )
    );
    let task: Vec<(Vec<u8>,)> = feature
        .dir
        .rows(
            "SELECT b.bytes FROM revision_file f JOIN blob b ON b.sha256 = f.blob_sha256 \
             JOIN artifact_revision r ON r.id = f.revision_id \
             WHERE r.number = 2 AND f.rel_path = 'tasks/A.md'",
        )
        .await;
    assert_eq!(task, [(b"# A, narrowed by hand\n".to_vec(),)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_invalid_edit_creates_no_revision_and_refuses() {
    let cycle = list(&[
        item("A", &["B"], "tasks/A.md"),
        item("B", &["A"], "tasks/B.md"),
    ]);
    let unknown_need = list(&[
        item("A", &["Z"], "tasks/A.md"),
        item("B", &[], "tasks/B.md"),
    ]);
    let duplicate = list(&[item("A", &[], "tasks/A.md"), item("A", &[], "tasks/B.md")]);
    let missing = list(&[
        item("A", &[], "tasks/A.md"),
        item("B", &[], "tasks/B.md"),
        item("C", &[], "tasks/C.md"),
    ]);
    let escaping = list(&[item("A", &[], "../A.md"), item("B", &[], "tasks/B.md")]);
    let cases: [(&str, &str, &str, &[&str]); 6] = [
        (
            "tasks/C.md",
            "# C\n",
            "plan.json: tasks/C.md is not referenced by any item",
            &["tasks/C.md"],
        ),
        (
            "plan.json",
            &cycle,
            "plan.json: needs form a cycle: A → B → A",
            &["plan.json"],
        ),
        (
            "plan.json",
            &unknown_need,
            "plan.json: item A needs Z, which is not an item",
            &["plan.json"],
        ),
        (
            "plan.json",
            &duplicate,
            "plan.json: key A is used by more than one item",
            &["plan.json"],
        ),
        (
            "plan.json",
            &missing,
            "tasks/C.md is missing",
            &["plan.json"],
        ),
        (
            "plan.json",
            &escaping,
            "../A.md is not a safe relative path",
            &["plan.json"],
        ),
    ];
    for (path, text, problem, files) in cases {
        let feature = plan_at_gate().await;
        feature.edit(path, text);

        let approve = feature.project.approve(&feature.run, None, ACTOR).await;

        assert_eq!(refusal(approve), invalid(problem, files), "{path}: {text}");
        assert_eq!(plan_revisions(&feature).await, [(1, 3)]);
        assert_eq!(
            feature.dir.event_data("working_copy.edit_refused").await,
            [serde_json::json!({
                "artifact": "plan",
                "reason": "invalid",
                "problem": problem,
                "files": files,
            })]
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_valid_edit_may_add_an_item_with_its_task_file() {
    let feature = plan_at_gate().await;
    let three = list(&[
        item("A", &[], "tasks/A.md"),
        item("B", &["A"], "tasks/B.md"),
        item("C", &["B"], "tasks/C.md"),
    ]);
    feature.edit("plan.json", &three);
    feature.edit("tasks/C.md", "# C\n");

    let approved = feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the edit is approved");

    assert_eq!(approved.revision, 2);
    assert_eq!(plan_revisions(&feature).await, [(1, 3), (2, 4)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_plan_removes_task_files_its_revision_no_longer_has() {
    let feature = plan_at_gate().await;
    feature
        .project
        .reject(&feature.run, None, "only A", ACTOR)
        .await
        .expect("the rejection is recorded");
    feature
        .drive(Script::new().on("plan", 2, 1, plan(&["A"])))
        .await
        .outcome
        .expect("the new plan reaches the gate");

    let approved = feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the new plan is approved");

    assert_eq!(
        (approved.revision, approved.origin),
        (2, RevisionOrigin::Agent)
    );
    assert!(!feature.working_copy("tasks/B.md").exists());
    assert!(feature.working_copy("tasks/A.md").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_edit_to_a_pinned_item_list_is_refused_by_every_command_that_acts() {
    let feature = plan_at_gate().await;
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the plan is approved");
    feature
        .drive(Script::new())
        .await
        .outcome
        .expect("the builder reads the plan");
    feature.edit("tasks/B.md", "# B, changed after building started\n");

    let approve = feature.project.approve(&feature.run, None, ACTOR).await;
    let driven = feature.drive(Script::new()).await;

    let pinned = Refusal::PinnedEdit {
        subject: SUBJECT.to_owned(),
        artifact: "plan".to_owned(),
        revision: 1,
        files: vec!["tasks/B.md".to_owned()],
    };
    assert_eq!(refusal(approve), pinned);
    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Halted(pinned)
    );
    assert_eq!(plan_revisions(&feature).await, [(1, 3)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_invalid_edit_while_the_planner_reruns_is_kept_and_halts_the_driver() {
    let feature = plan_at_gate().await;
    feature
        .project
        .reject(&feature.run, None, "smaller", ACTOR)
        .await
        .expect("the rejection is recorded");
    let runtime = Editing::new(
        Script::new().on("plan", 2, 1, plan(&["A"])),
        "plan",
        feature.working_copy("plan.json"),
        "not json",
    );

    let driven = feature.drive_with(&runtime).await;

    assert!(matches!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Halted(Refusal::InvalidEdit { .. })
    ));
    assert_eq!(
        fs::read_to_string(feature.working_copy("plan.json")).expect("the edit is kept"),
        "not json"
    );
    assert_eq!(plan_revisions(&feature).await, [(1, 3)]);
    let ended: Vec<(String,)> = feature
        .dir
        .rows(
            "SELECT e.state FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             WHERE s.step_key = 'plan' AND s.iteration = 2",
        )
        .await;
    assert_eq!(ended, [("interrupted".to_owned(),)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_edit_after_approval_holds_the_per_item_expansion_until_it_is_approved() {
    let feature = approved(TASK_WORKFLOW, &[("A", &[]), ("B", &[])]).await;
    feature.edit("tasks/A.md", "# Task A, edited after approval\n");

    let driven = feature
        .drive(Script::new().on("A/build", 1, 1, work("a")))
        .await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Halted(Refusal::UnapprovedRevision {
            subject: SUBJECT.to_owned(),
            artifact: "plan".to_owned(),
            revision: 2,
        })
    );
    let children: Vec<(i64,)> = feature
        .dir
        .rows("SELECT count(*) FROM workflow_run WHERE parent_step_run_id IS NOT NULL")
        .await;
    assert_eq!(children, [(0,)]);
}
