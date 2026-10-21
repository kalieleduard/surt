use super::{FRAME_MS, frame};
use crate::symbols::{ASCII_SPINNER, SPINNER, Symbols};

const EPOCH_MS: i64 = 1_700_000_000_000;
const MODES: [Symbols; 2] = [Symbols::Unicode, Symbols::Ascii];

fn cycle_ms(symbols: Symbols) -> i64 {
    symbols.spinner().len() as i64 * FRAME_MS
}

#[test]
fn the_first_frame_lasts_until_the_frame_duration_is_reached() {
    assert_eq!(frame(0, Symbols::Unicode), SPINNER[0]);
    assert_eq!(frame(FRAME_MS - 1, Symbols::Unicode), SPINNER[0]);
    assert_eq!(frame(FRAME_MS, Symbols::Unicode), SPINNER[1]);
}

#[test]
fn frames_follow_the_spinner_of_each_mode_in_order() {
    let frames = |symbols: Symbols| -> Vec<&str> {
        (0..symbols.spinner().len() as i64)
            .map(|step| frame(step * FRAME_MS, symbols))
            .collect()
    };

    assert_eq!(frames(Symbols::Unicode), SPINNER);
    assert_eq!(frames(Symbols::Ascii), ASCII_SPINNER);
}

#[test]
fn the_ascii_spinner_is_ascii_and_bounces_a_bar_back_and_forth() {
    assert!(ASCII_SPINNER.iter().all(|frame| frame.is_ascii()));
    assert_eq!(frame(3 * FRAME_MS, Symbols::Ascii), "[   =]");
    assert_eq!(frame(4 * FRAME_MS, Symbols::Ascii), "[  = ]");
}

#[test]
fn consecutive_steps_always_give_different_frames() {
    for symbols in MODES {
        for step in 0..(symbols.spinner().len() as i64 * 3) {
            assert_ne!(
                frame(step * FRAME_MS, symbols),
                frame((step + 1) * FRAME_MS, symbols)
            );
        }
    }
}

#[test]
fn the_cycle_repeats_after_all_frames_were_shown() {
    for symbols in MODES {
        let cycle = cycle_ms(symbols);
        for elapsed in [0, 1, FRAME_MS, 250, cycle - 1] {
            assert_eq!(frame(elapsed, symbols), frame(elapsed + cycle, symbols));
        }
    }
}

#[test]
fn extreme_times_pick_a_frame_without_panicking() {
    for symbols in MODES {
        let frames = symbols.spinner();
        for elapsed in [EPOCH_MS, i64::MAX, -1, i64::MIN] {
            assert!(frames.contains(&frame(elapsed, symbols)));
        }
        assert_eq!(frame(-1, symbols), frames[frames.len() - 1]);
    }
}
