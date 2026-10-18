use std::path::Path;

use super::evidence::change;
use super::repository::commit_file;
use super::team::{
    REVIEWED_TASK, Team, blocking, builds, built, failures, input, input_line, reviews,
    reviews_commit, reviews_with,
};
use crate::project::{Config, DriveOutcome};

fn rework(worktree: &Path) -> String {
    commit_file(worktree, "feature.txt", "the reworked feature\n")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_review_of_another_commit_is_an_invalid_delivery() {
    let feature = built(REVIEWED_TASK, &[("A", &[])], Config::default()).await;
    let team = Team::new()
        .on("A/build", 1, 1, builds(change))
        .on("A/review", 1, 1, reviews_commit("approved", feature.base()))
        .on("A/review", 1, 2, reviews("approved"));

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let reviewed = feature.task_tip("A").await;
    assert_eq!(
        failures(&feature, "review").await,
        [(
            1,
            1,
            "invalid_delivery".to_owned(),
            format!(
                "feedback: reviewed_commit {} is not the commit under review {reviewed}",
                feature.base()
            )
        )]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_approval_with_a_blocking_finding_is_an_invalid_delivery() {
    let feature = built(REVIEWED_TASK, &[("A", &[])], Config::default()).await;
    let team = Team::new()
        .on("A/build", 1, 1, builds(change))
        .on(
            "A/review",
            1,
            1,
            reviews_with("approved", vec![blocking("feature.txt")]),
        )
        .on("A/review", 1, 2, reviews("approved"));

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(
        failures(&feature, "review").await,
        [(
            1,
            1,
            "invalid_delivery".to_owned(),
            "feedback: approved with a blocking finding".to_owned()
        )]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn changes_requested_start_an_iteration_whose_builder_and_reviewer_read_the_feedback() {
    let feature = built(REVIEWED_TASK, &[("A", &[])], Config::default()).await;
    let team = Team::new()
        .on("A/build", 1, 1, builds(change))
        .on(
            "A/review",
            1,
            1,
            reviews_with("changes_requested", vec![blocking("feature.txt")]),
        )
        .on("A/build", 2, 1, builds(rework))
        .on("A/review", 2, 1, reviews("approved"));

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert!(failures(&feature, "review").await.is_empty());
    let first = team.request("A/review", 1);
    assert_eq!(input_line(&first, "feedback"), None);
    for step in ["A/build", "A/review"] {
        let second = team.request(step, 2);
        assert_eq!(
            input_line(&second, "feedback").as_deref(),
            Some("feedback (iteration 1, review)"),
            "{step}"
        );
        let feedback = input(&second, "feedback", "feedback.json").expect("the feedback");
        assert_eq!(feedback["findings"][0]["blocking"], true, "{step}");
    }
}
