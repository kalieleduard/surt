use super::{FRAME_MS, frame};
use crate::symbols::SPINNER;

const CYCLE_MS: i64 = SPINNER.len() as i64 * FRAME_MS;
const EPOCH_MS: i64 = 1_700_000_000_000;

#[test]
fn the_first_frame_lasts_until_the_frame_duration_is_reached() {
    assert_eq!(frame(0), SPINNER[0]);
    assert_eq!(frame(FRAME_MS - 1), SPINNER[0]);
    assert_eq!(frame(FRAME_MS), SPINNER[1]);
}

#[test]
fn frames_follow_the_symbol_table_in_order() {
    let frames: Vec<&str> = (0..SPINNER.len() as i64)
        .map(|step| frame(step * FRAME_MS))
        .collect();

    assert_eq!(frames, SPINNER);
}

#[test]
fn consecutive_steps_always_give_different_frames() {
    for step in 0..(SPINNER.len() as i64 * 3) {
        assert_ne!(frame(step * FRAME_MS), frame((step + 1) * FRAME_MS));
    }
}

#[test]
fn the_cycle_repeats_after_all_frames_were_shown() {
    for elapsed in [0, 1, FRAME_MS, 250, CYCLE_MS - 1] {
        assert_eq!(frame(elapsed), frame(elapsed + CYCLE_MS));
    }
}

#[test]
fn an_epoch_timestamp_does_not_panic() {
    assert!(SPINNER.contains(&frame(EPOCH_MS)));
    assert!(SPINNER.contains(&frame(i64::MAX)));
}

#[test]
fn a_negative_time_picks_a_frame_without_panicking() {
    assert!(SPINNER.contains(&frame(-1)));
    assert!(SPINNER.contains(&frame(i64::MIN)));
    assert_eq!(frame(-1), SPINNER[SPINNER.len() - 1]);
}
