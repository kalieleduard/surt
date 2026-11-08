use std::fs;

use super::super::fixtures::SUBJECT;
use super::fixtures::Feature;
use super::inspecting::reworked;
use crate::project::{
    DefinedStep, DefinedStepKind, Error, InspectRequest, Refusal, RunSelector, StepCondition,
};

fn whole_run() -> InspectRequest<'static> {
    InspectRequest {
        item: None,
        run: RunSelector::Current,
    }
}

fn task(key: &str) -> InspectRequest<'_> {
    InspectRequest {
        item: Some(key),
        run: RunSelector::Current,
    }
}

fn condition(step: &str, outcome: &str) -> StepCondition {
    StepCondition {
        step: step.to_owned(),
        outcome: outcome.to_owned(),
    }
}

fn step(key: &str, needs: &[&str], kind: DefinedStepKind) -> DefinedStep {
    DefinedStep {
        key: key.to_owned(),
        needs: needs.iter().map(|need| (*need).to_owned()).collect(),
        when: None,
        kind,
    }
}

fn names(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|key| (*key).to_owned()).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_run_definition_describes_each_step_its_kind_and_dependencies() {
    let feature = Feature::review().await;

    let definition = feature
        .project
        .frozen_definition(SUBJECT, whole_run())
        .await
        .expect("the definition is read");

    assert_eq!(definition.name, "review");
    let keys: Vec<&str> = definition
        .steps
        .iter()
        .map(|step| step.key.as_str())
        .collect();
    assert_eq!(keys, ["write", "approve", "drafting", "publish"]);
    assert_eq!(
        definition.steps[1],
        step(
            "approve",
            &["write"],
            DefinedStepKind::Approval {
                artifact: "draft".to_owned()
            }
        )
    );
    assert_eq!(
        definition.steps[2].kind,
        DefinedStepKind::Loop {
            body: names(&["write", "approve"]),
            until: condition("approve", "approved"),
        }
    );
    assert!(definition.source.contains("[steps.publish]"));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_run_definition_stays_frozen_when_the_project_file_changes() {
    let feature = Feature::review().await;
    let file = feature.dir.path().join(".surt/workflows/review.toml");
    let edited = fs::read_to_string(&file)
        .expect("the workflow file")
        .replace("[steps.publish]", "[steps.announce]");
    fs::write(&file, edited).expect("the workflow file is edited");

    let definition = feature
        .project
        .frozen_definition(SUBJECT, whole_run())
        .await
        .expect("the definition is read");

    assert_eq!(
        definition.steps.last().map(|step| step.key.as_str()),
        Some("publish")
    );
    assert!(!definition.source.contains("announce"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_task_definition_is_the_child_workflow_its_per_item_step_runs() {
    let feature = reworked().await;

    let root = feature
        .project
        .frozen_definition(SUBJECT, whole_run())
        .await
        .expect("the run definition is read");
    let child = feature
        .project
        .frozen_definition(SUBJECT, task("A"))
        .await
        .expect("the task definition is read");

    assert_eq!(
        root.steps.last().map(|step| &step.kind),
        Some(&DefinedStepKind::PerItem {
            items: "plan".to_owned(),
            workflow: "task".to_owned(),
        })
    );
    assert_eq!(child.name, "task");
    let review = child
        .steps
        .iter()
        .find(|step| step.key == "review")
        .expect("the review step");
    assert_eq!(review.needs, names(&["check"]));
    assert_eq!(review.when, Some(condition("check", "passed")));
    assert_eq!(
        review.kind,
        DefinedStepKind::Agent {
            agent: "reviewer".to_owned()
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_task_is_refused_with_the_known_ones() {
    let feature = reworked().await;

    let refused = feature
        .project
        .frozen_definition(SUBJECT, task("Z"))
        .await
        .expect_err("an unknown task is refused");

    assert!(
        matches!(
            &refused,
            Error::Refused(Refusal::UnknownItem { item, known, .. })
                if item == "Z" && known == &names(&["A"])
        ),
        "{refused:?}"
    );
}
