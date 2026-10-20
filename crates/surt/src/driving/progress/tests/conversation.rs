use surt_engine::project::{Progress, StepLabel};
use surt_engine::runtime::RestartReason;

use super::line;

fn spec(iteration: Option<u32>) -> StepLabel {
    StepLabel {
        key: "spec".to_owned(),
        item: None,
        iteration,
    }
}

fn launched(iteration: u32, attempt: u32, continued: bool) -> Progress {
    Progress::StepStarted {
        step: spec(Some(iteration)),
        attempt,
        continued,
    }
}

#[test]
fn a_continued_launch_names_its_iteration_and_the_same_conversation() {
    let lines: String = [launched(1, 1, false), launched(2, 1, true)]
        .iter()
        .map(line)
        .collect();

    assert_eq!(
        lines,
        "● spec started\n● spec continued (iteration 2, same conversation)\n"
    );
}

#[test]
fn a_continued_retry_still_reads_as_the_next_attempt() {
    assert_eq!(line(&launched(2, 2, true)), "↻ spec iter 2 attempt 2\n");
}

#[test]
fn a_restarted_conversation_names_its_iteration_and_why() {
    let restarted = |reason| Progress::ConversationRestarted {
        step: spec(Some(3)),
        reason,
    };

    let lines: String = [
        restarted(RestartReason::TranscriptMissing),
        restarted(RestartReason::ContinuationRefused),
    ]
    .iter()
    .map(line)
    .collect();

    assert_eq!(
        lines,
        "↻ spec restarted (iteration 3): transcript_missing\n\
         ↻ spec restarted (iteration 3): continuation_refused\n"
    );
}

#[test]
fn an_item_step_outside_a_loop_continues_at_its_first_iteration() {
    let step = StepLabel {
        key: "build".to_owned(),
        item: Some("B".to_owned()),
        iteration: None,
    };

    let continued = line(&Progress::StepStarted {
        step,
        attempt: 1,
        continued: true,
    });

    assert_eq!(
        continued,
        "● item B build continued (iteration 1, same conversation)\n"
    );
}
