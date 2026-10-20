use super::conversations::{continuing, reject, rejected_after};
use super::fixtures::{draft, label};
use crate::project::Progress;
use crate::runtime::{Behavior, Delivery, RestartReason, Script};

fn launches(progress: &[Progress]) -> Vec<Progress> {
    progress
        .iter()
        .filter(|event| {
            matches!(
                event,
                Progress::StepStarted { .. } | Progress::ConversationRestarted { .. }
            )
        })
        .cloned()
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_launch_says_whether_it_continues_the_previous_conversation() {
    let feature = continuing().await;
    let first = feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await;
    reject(&feature).await;

    let second = feature
        .drive(Script::new().on("write", 2, 1, draft("# Draft 2\n")))
        .await;

    assert_eq!(
        launches(&first.progress),
        [Progress::StepStarted {
            step: label("write", Some(1)),
            attempt: 1,
            continued: false,
        }]
    );
    assert_eq!(
        launches(&second.progress),
        [Progress::StepStarted {
            step: label("write", Some(2)),
            attempt: 1,
            continued: true,
        }]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_lost_transcript_reports_the_restart_after_the_continued_launch() {
    let feature = continuing().await;
    rejected_after(&feature, 1).await;
    let lost = Behavior::TranscriptMissing(Delivery::new("done").output(
        "draft",
        "draft.md",
        "# Draft 2\n",
    ));

    let driven = feature.drive(Script::new().on("write", 2, 1, lost)).await;

    assert_eq!(
        launches(&driven.progress),
        [
            Progress::StepStarted {
                step: label("write", Some(2)),
                attempt: 1,
                continued: true,
            },
            Progress::ConversationRestarted {
                step: label("write", Some(2)),
                reason: RestartReason::TranscriptMissing,
            },
        ]
    );
}
