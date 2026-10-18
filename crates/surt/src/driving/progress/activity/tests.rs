use surt_engine::project::{ActivitySnapshot, RunningStep, StepLabel};

use super::{Activity, Output, Terminal, on_tick};
use crate::symbols::Symbols;

const STARTED: i64 = 1_000_000;
const MINUTE: i64 = 60;

fn running(item: Option<&str>, key: &str, iteration: Option<u32>, started_at: i64) -> RunningStep {
    RunningStep {
        step: StepLabel {
            key: key.to_owned(),
            item: item.map(str::to_owned),
            iteration,
        },
        started_at,
    }
}

fn busy() -> ActivitySnapshot {
    ActivitySnapshot {
        running: vec![
            running(Some("B"), "build", None, STARTED + 26 * MINUTE),
            running(Some("C"), "review", Some(1), STARTED + 31 * MINUTE),
        ],
        queued: 1,
        slots_in_use: 3,
        slots_limit: 3,
    }
}

fn activity(snapshot: ActivitySnapshot, last_line: i64) -> Activity {
    Activity {
        snapshot: Some(snapshot),
        started: STARTED,
        last_line,
    }
}

#[test]
fn the_footer_shows_what_runs_for_how_long_the_queue_the_slots_and_the_elapsed_time() {
    let now = STARTED + 32 * MINUTE + 10;

    let footer = on_tick(&activity(busy(), STARTED), now, Terminal::Tty);

    assert_eq!(
        footer,
        Some(Output::Footer(
            "── 2 running (B build 6m, C review 1m) · 1 queued · slots 3/3 · 32m ──".to_owned()
        ))
    );
}

#[test]
fn the_footer_names_root_steps_and_later_iterations_and_counts_seconds_then_hours() {
    let snapshot = ActivitySnapshot {
        running: vec![running(None, "spec", Some(2), STARTED + 2 * 3600)],
        queued: 0,
        slots_in_use: 1,
        slots_limit: 3,
    };
    let now = STARTED + 2 * 3600 + 42;

    let footer = on_tick(&activity(snapshot, STARTED), now, Terminal::Tty);

    assert_eq!(
        footer,
        Some(Output::Footer(
            "── 1 running (spec iter 2 42s) · slots 1/3 · 2h00m ──".to_owned()
        ))
    );
}

#[test]
fn there_is_no_footer_before_the_first_snapshot() {
    let none = Activity {
        snapshot: None,
        started: STARTED,
        last_line: STARTED,
    };

    assert_eq!(on_tick(&none, STARTED + 10 * MINUTE, Terminal::Tty), None);
}

#[test]
fn without_a_tty_a_line_fires_once_five_minutes_pass_without_an_event_line() {
    let last_line = STARTED + 31 * MINUTE;
    let quiet = activity(busy(), last_line);

    assert_eq!(
        on_tick(&quiet, last_line + 5 * MINUTE - 1, Terminal::Plain),
        None
    );
    assert_eq!(
        on_tick(&quiet, last_line + 5 * MINUTE, Terminal::Plain),
        Some(Output::Line(
            "… still running: B build (10m), C review (5m)".to_owned()
        ))
    );
}

#[test]
fn without_a_tty_nothing_fires_when_nothing_runs() {
    let idle = ActivitySnapshot {
        running: Vec::new(),
        queued: 0,
        slots_in_use: 0,
        slots_limit: 3,
    };

    assert_eq!(
        on_tick(
            &activity(idle, STARTED),
            STARTED + 60 * MINUTE,
            Terminal::Plain
        ),
        None
    );
}

fn ascii(output: Option<Output>) -> String {
    match output.expect("an output") {
        Output::Footer(text) | Output::Line(text) => Symbols::Ascii.render(&text).into_owned(),
    }
}

#[test]
fn the_footer_and_the_heartbeat_line_print_only_ascii_in_ascii_mode() {
    let now = STARTED + 32 * MINUTE + 10;

    let footer = ascii(on_tick(&activity(busy(), STARTED), now, Terminal::Tty));
    let heartbeat = ascii(on_tick(&activity(busy(), STARTED), now, Terminal::Plain));

    assert_eq!(
        footer,
        "-- 2 running (B build 6m, C review 1m) | 1 queued | slots 3/3 | 32m --"
    );
    assert_eq!(heartbeat, "... still running: B build (6m), C review (1m)");
}
