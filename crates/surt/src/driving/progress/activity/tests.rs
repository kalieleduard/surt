use surt_engine::project::{ActivityPause, ActivitySnapshot, RunningStep, StepLabel};

use super::{Activity, Output, Terminal, on_tick};
use crate::driving::local_time;
use crate::symbols::Symbols;

const STARTED: i64 = 1_000_000;
const MINUTE: i64 = 60;
const WIDE: Terminal = Terminal::Tty { width: 200 };

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
        paused: None,
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

    let footer = on_tick(&activity(busy(), STARTED), now, WIDE);

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
        paused: None,
    };
    let now = STARTED + 2 * 3600 + 42;

    let footer = on_tick(&activity(snapshot, STARTED), now, WIDE);

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

    assert_eq!(on_tick(&none, STARTED + 10 * MINUTE, WIDE), None);
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
        paused: None,
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

    let footer = ascii(on_tick(&activity(busy(), STARTED), now, WIDE));
    let heartbeat = ascii(on_tick(&activity(busy(), STARTED), now, Terminal::Plain));

    assert_eq!(
        footer,
        "-- 2 running (B build 6m, C review 1m) | 1 queued | slots 3/3 | 32m --"
    );
    assert_eq!(heartbeat, "... still running: B build (6m), C review (1m)");
}

#[test]
fn a_footer_wider_than_the_terminal_is_cut_to_stay_on_one_line() {
    let now = STARTED + 32 * MINUTE + 10;
    let footer = |width| on_tick(&activity(busy(), STARTED), now, Terminal::Tty { width });

    assert_eq!(
        footer(30),
        Some(Output::Footer("── 2 running (B build 6m,…".to_owned()))
    );
    assert_eq!(
        footer(80),
        Some(Output::Footer(
            "── 2 running (B build 6m, C review 1m) · 1 queued · slots 3/3 · 32m ──".to_owned()
        ))
    );
}

const RESET: i64 = 1_792_900_800;

fn paused(running: Vec<RunningStep>, waiting: usize, backoff: Option<i64>) -> ActivitySnapshot {
    ActivitySnapshot {
        running,
        queued: waiting,
        slots_in_use: 0,
        slots_limit: 3,
        paused: Some(ActivityPause {
            until: RESET,
            backoff,
        }),
    }
}

#[test]
fn the_footer_says_until_when_launches_are_paused_and_how_many_agents_wait() {
    let at = local_time(RESET).expect("a local time");
    let now = STARTED + 32 * MINUTE + 10;
    let footer = |snapshot| on_tick(&activity(snapshot, STARTED), now, WIDE);

    assert_eq!(
        footer(paused(Vec::new(), 2, None)),
        Some(Output::Footer(format!(
            "── 0 running · ⏸ paused until {at} · 2 waiting · slots 0/3 · 32m ──"
        )))
    );
    assert_eq!(
        footer(paused(Vec::new(), 1, Some(120))),
        Some(Output::Footer(format!(
            "── 0 running · ⏸ paused until {at} (backoff 2m) · 1 waiting · slots 0/3 · 32m ──"
        )))
    );
}

#[test]
fn without_a_tty_the_heartbeat_mentions_the_pause_even_when_nothing_runs() {
    let at = local_time(RESET).expect("a local time");
    let last_line = STARTED + 31 * MINUTE;
    let quiet = |snapshot| {
        on_tick(
            &activity(snapshot, last_line),
            last_line + 5 * MINUTE,
            Terminal::Plain,
        )
    };
    let building = vec![running(Some("B"), "build", None, STARTED + 26 * MINUTE)];

    assert_eq!(
        quiet(paused(Vec::new(), 2, None)),
        Some(Output::Line(format!(
            "… agent launches paused until {at} — 2 waiting"
        )))
    );
    assert_eq!(
        quiet(paused(building, 1, None)),
        Some(Output::Line(format!(
            "… still running: B build (10m) · agent launches paused until {at} — 1 waiting"
        )))
    );
}

#[test]
fn the_paused_footer_reads_in_ascii_mode() {
    let at = local_time(RESET).expect("a local time");
    let now = STARTED + 32 * MINUTE + 10;

    let footer = ascii(on_tick(
        &activity(paused(Vec::new(), 2, None), STARTED),
        now,
        WIDE,
    ));

    assert_eq!(
        footer,
        format!("-- 0 running | [paused] paused until {at} | 2 waiting | slots 0/3 | 32m --")
    );
}

#[test]
fn the_paused_heartbeat_line_reads_in_ascii_mode() {
    let at = local_time(RESET).expect("a local time");
    let last_line = STARTED + 31 * MINUTE;
    let building = vec![running(Some("B"), "build", None, STARTED + 26 * MINUTE)];

    let heartbeat = ascii(on_tick(
        &activity(paused(building, 1, None), last_line),
        last_line + 5 * MINUTE,
        Terminal::Plain,
    ));

    assert_eq!(
        heartbeat,
        format!("... still running: B build (10m) | agent launches paused until {at} - 1 waiting")
    );
}
