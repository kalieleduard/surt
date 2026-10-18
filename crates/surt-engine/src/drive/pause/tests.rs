use super::Pause;

const NOW: i64 = 1_800_000_000;
const MINUTE: i64 = 60;

#[derive(Clone, Copy)]
enum Ending {
    Transient(Option<i64>),
    Other,
}

use Ending::{Other, Transient};

fn after(endings: &[(i64, Ending)]) -> (Option<i64>, Option<u32>) {
    let pause = endings
        .iter()
        .fold(Pause::default(), |pause, &(at, ending)| match ending {
            Transient(resets_at) => pause.failed(resets_at, at),
            Other => pause.ended(),
        });
    let last = endings.last().map_or(NOW, |&(at, _)| at);
    (pause.until(last), pause.backoff_step())
}

#[test]
fn a_driver_that_saw_no_transient_failure_is_not_paused() {
    assert_eq!(after(&[]), (None, None));
    assert_eq!(after(&[(NOW, Other)]), (None, None));
}

#[test]
fn a_reset_time_in_the_future_ends_the_pause_at_that_time() {
    let reset = NOW + 3 * 60 * MINUTE;

    assert_eq!(after(&[(NOW, Transient(Some(reset)))]), (Some(reset), None));
}

#[test]
fn a_reset_time_that_is_not_in_the_future_counts_as_no_reset_time() {
    let table = [("in the past", NOW - 10), ("right now", NOW)];

    for (case, reset) in table {
        assert_eq!(
            after(&[(NOW, Transient(Some(reset)))]),
            (Some(NOW + MINUTE), Some(1)),
            "{case}"
        );
    }
}

#[test]
fn without_a_reset_time_each_failed_resume_backs_off_1_2_4_8_then_15_minutes() {
    let delays = [1, 2, 4, 8, 15, 15, 15];
    let mut endings = Vec::new();
    let mut at = NOW;

    for (step, minutes) in (1..).zip(delays) {
        endings.push((at, Transient(None)));
        let until = at + minutes * MINUTE;

        assert_eq!(after(&endings), (Some(until), Some(step)), "step {step}");
        at = until;
    }
}

#[test]
fn with_several_reset_times_the_latest_one_wins() {
    let table = [
        ("an earlier reset", NOW + 200, NOW + 300),
        ("a later reset", NOW + 400, NOW + 400),
    ];

    for (case, second, expected) in table {
        let endings = [
            (NOW, Transient(Some(NOW + 300))),
            (NOW + 1, Transient(Some(second))),
        ];

        assert_eq!(after(&endings), (Some(expected), None), "{case}");
    }
}

#[test]
fn a_reset_time_extends_a_backoff_and_a_backoff_never_shortens_a_reset() {
    let table = [
        (
            "a reset during a backoff",
            [
                (NOW, Transient(None)),
                (NOW + 5, Transient(Some(NOW + 600))),
            ],
            (Some(NOW + 600), None),
        ),
        (
            "an earlier reset during a backoff",
            [(NOW, Transient(None)), (NOW + 5, Transient(Some(NOW + 30)))],
            (Some(NOW + MINUTE), Some(1)),
        ),
        (
            "no reset during a reset pause",
            [
                (NOW, Transient(Some(NOW + 600))),
                (NOW + 5, Transient(None)),
            ],
            (Some(NOW + 600), None),
        ),
    ];

    for (case, endings, expected) in table {
        assert_eq!(after(&endings), expected, "{case}");
    }
}

#[test]
fn failures_of_agents_already_running_during_the_pause_do_not_advance_the_step() {
    let during = [(NOW, Transient(None)), (NOW + 10, Transient(None))];
    let then_resumed = [
        (NOW, Transient(None)),
        (NOW + 10, Transient(None)),
        (NOW + MINUTE, Transient(None)),
    ];

    assert_eq!(after(&during), (Some(NOW + MINUTE), Some(1)));
    assert_eq!(
        after(&then_resumed),
        (Some(NOW + 3 * MINUTE), Some(2)),
        "the resume that failed takes the second step"
    );
}

#[test]
fn a_reset_time_leaves_the_backoff_step_where_it_was() {
    let endings = [
        (NOW, Transient(None)),
        (NOW + MINUTE, Transient(Some(NOW + 600))),
        (NOW + 600, Transient(None)),
    ];

    assert_eq!(after(&endings), (Some(NOW + 600 + 2 * MINUTE), Some(2)));
}

#[test]
fn the_backoff_starts_over_once_an_agent_execution_ends_any_other_way() {
    let endings = [
        (NOW, Transient(None)),
        (NOW + MINUTE, Transient(None)),
        (NOW + 200, Other),
        (NOW + 300, Transient(None)),
    ];

    assert_eq!(after(&endings), (Some(NOW + 300 + MINUTE), Some(1)));
}

#[test]
fn an_agent_ending_otherwise_during_the_pause_does_not_end_it() {
    let endings = [(NOW, Transient(Some(NOW + 600))), (NOW + 5, Other)];

    assert_eq!(after(&endings), (Some(NOW + 600), None));
}

#[test]
fn the_pause_is_over_once_its_end_is_reached() {
    let pause = Pause::default().failed(Some(NOW + 30), NOW);

    assert_eq!(pause.until(NOW + 29), Some(NOW + 30));
    assert_eq!(pause.until(NOW + 30), None);
}
