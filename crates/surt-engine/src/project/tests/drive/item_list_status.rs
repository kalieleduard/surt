use super::super::fixtures::SUBJECT;
use super::fixtures::{Feature, draft};
use super::planned::{TASK_WORKFLOW, plan, planned_catalog};
use crate::project::{ItemListSummary, RunSelector, RunStatus, StatusScope};
use crate::runtime::Script;

async fn status(feature: &Feature) -> RunStatus {
    feature
        .project
        .status(StatusScope::Subject(SUBJECT, RunSelector::Current))
        .await
        .expect("the status reads")
        .remove(0)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_gate_on_an_item_list_counts_its_items_and_item_files_from_the_revision() {
    let feature = Feature::new(planned_catalog(TASK_WORKFLOW), "planned").await;
    feature
        .drive(Script::new().on("plan", 1, 1, plan(&[("A", &[]), ("B", &["A"])])))
        .await;

    let status = status(&feature).await;

    let revision = status.gates[0].revision.as_ref().expect("a revision waits");
    assert_eq!(
        revision.item_list,
        Some(ItemListSummary {
            items: 2,
            item_files: 2,
        })
    );
    assert_eq!(status.item_step.as_deref(), Some("implement"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_gate_on_a_plain_artifact_has_no_item_list_and_a_run_without_per_item_has_no_item_step() {
    let feature = Feature::review().await;
    feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await;

    let status = status(&feature).await;

    let revision = status.gates[0].revision.as_ref().expect("a revision waits");
    assert_eq!(revision.files, ["draft.md"]);
    assert_eq!(revision.item_list, None);
    assert_eq!(status.item_step, None);
}
