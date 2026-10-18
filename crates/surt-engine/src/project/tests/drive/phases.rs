use super::fixtures::{ACTOR, Feature, draft};
use crate::project::GateStatus;
use crate::runtime::Script;

const GATE: &str = "approve";

async fn status(feature: &Feature) -> GateStatus {
    feature
        .project
        .gate_status(&feature.run, GATE)
        .await
        .expect("the gate status")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_gate_reports_each_decision_on_the_revision_it_covered() {
    let feature = Feature::review().await;
    let script = Script::new().on("write", 1, 1, draft("# Draft 1\n")).on(
        "write",
        2,
        1,
        draft("# Draft 2\n"),
    );
    let before = status(&feature).await;
    feature.drive(script.clone()).await;
    let first = status(&feature).await;
    feature
        .project
        .reject(&feature.run, None, "shorter", ACTOR)
        .await
        .expect("the rejection");
    let rejected = status(&feature).await;
    feature.drive(script).await;
    let second = status(&feature).await;
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the approval");

    let approved = status(&feature).await;

    assert_eq!(before, GateStatus::NotReached);
    assert_eq!(first, GateStatus::Waiting { revision: 1 });
    assert_eq!(rejected, GateStatus::Rejected { revision: 1 });
    assert_eq!(second, GateStatus::Waiting { revision: 2 });
    assert_eq!(approved, GateStatus::Approved { revision: 2 });
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_the_workflow_does_not_declare_was_never_reached() {
    let feature = Feature::review().await;

    let unknown = feature
        .project
        .gate_status(&feature.run, "approve-tasks")
        .await
        .expect("the gate status");

    assert_eq!(unknown, GateStatus::NotReached);
}
