use surt_engine::project::{ActivityPause, ActivitySnapshot, RunningStep, StepLabel};

use super::{Activity, Output, Terminal, on_tick};
use crate::driving::local_time;
use crate::symbols::{SPIN_1, SPIN_2, SPIN_3, SPIN_4, SPINNER, Symbols};

const STARTED: i64 = 1_000_000;
const MINUTE: i64 = 60;
const WIDE: Terminal = Terminal::Tty { width: 200 };

// Even seconds are multiples of 400 ms, so they land on the first frame.
fn ms(seconds: i64) -> i64 {
    seconds * 1000
}

fn spun(text: &str) -> String {
    format!("{SPIN_1} {text}")
}

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

fn nothing_yet() -> Activity {
    Activity {
        snapshot: None,
        started: STARTED,
        last_line: STARTED,
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

    let footer = on_tick(&activity(busy(), STARTED), ms(now), WIDE);

    assert_eq!(
        footer,
        Some(Output::Footer(spun(
            "── 2 running (B build 6m, C review 1m) · 1 queued · slots 3/3 · 32m ──"
        )))
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

    let footer = on_tick(&activity(snapshot, STARTED), ms(now), WIDE);

    assert_eq!(
        footer,
        Some(Output::Footer(spun(
            "── 1 running (spec iter 2 42s) · slots 1/3 · 2h00m ──"
        )))
    );
}

#[test]
fn before_the_first_snapshot_the_footer_is_the_spinner_and_the_total_time() {
    let now = ms(STARTED + 10 * MINUTE);

    assert_eq!(
        on_tick(&nothing_yet(), now, WIDE),
        Some(Output::Footer(spun("── 10m ──")))
    );
}

#[test]
fn without_a_tty_there_is_no_output_before_the_first_snapshot() {
    let now = ms(STARTED + 10 * MINUTE);

    assert_eq!(on_tick(&nothing_yet(), now, Terminal::Plain), None);
    assert_eq!(
        on_tick(&nothing_yet(), ms(STARTED + 60 * MINUTE), Terminal::Plain),
        None
    );
}

#[test]
fn without_a_tty_a_line_fires_once_five_minutes_pass_without_an_event_line() {
    let last_line = STARTED + 31 * MINUTE;
    let quiet = activity(busy(), last_line);

    assert_eq!(
        on_tick(&quiet, ms(last_line + 5 * MINUTE) - 1, Terminal::Plain),
        None
    );
    assert_eq!(
        on_tick(&quiet, ms(last_line + 5 * MINUTE), Terminal::Plain),
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
            ms(STARTED + 60 * MINUTE),
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

    let footer = ascii(on_tick(&activity(busy(), STARTED), ms(now), WIDE));
    let heartbeat = ascii(on_tick(
        &activity(busy(), STARTED),
        ms(now),
        Terminal::Plain,
    ));

    assert_eq!(
        footer,
        "| -- 2 running (B build 6m, C review 1m) | 1 queued | slots 3/3 | 32m --"
    );
    assert_eq!(heartbeat, "... still running: B build (6m), C review (1m)");
}

#[test]
fn a_footer_wider_than_the_terminal_is_cut_to_stay_on_one_line() {
    let now = STARTED + 32 * MINUTE + 10;
    let footer = |width| on_tick(&activity(busy(), STARTED), ms(now), Terminal::Tty { width });

    assert_eq!(
        footer(30),
        Some(Output::Footer(spun("── 2 running (B build 6m…")))
    );
    assert_eq!(
        footer(80),
        Some(Output::Footer(spun(
            "── 2 running (B build 6m, C review 1m) · 1 queued · slots 3/3 · 32m ──"
        )))
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
    let footer = |snapshot| on_tick(&activity(snapshot, STARTED), ms(now), WIDE);

    assert_eq!(
        footer(paused(Vec::new(), 2, None)),
        Some(Output::Footer(spun(&format!(
            "── 0 running · ⏸ paused until {at} · 2 waiting · slots 0/3 · 32m ──"
        ))))
    );
    assert_eq!(
        footer(paused(Vec::new(), 1, Some(120))),
        Some(Output::Footer(spun(&format!(
            "── 0 running · ⏸ paused until {at} (backoff 2m) · 1 waiting · slots 0/3 · 32m ──"
        ))))
    );
}

#[test]
fn without_a_tty_the_heartbeat_mentions_the_pause_even_when_nothing_runs() {
    let at = local_time(RESET).expect("a local time");
    let last_line = STARTED + 31 * MINUTE;
    let quiet = |snapshot| {
        on_tick(
            &activity(snapshot, last_line),
            ms(last_line + 5 * MINUTE),
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
        ms(now),
        WIDE,
    ));

    assert_eq!(
        footer,
        format!("| -- 0 running | [paused] paused until {at} | 2 waiting | slots 0/3 | 32m --")
    );
}

#[test]
fn the_paused_heartbeat_line_reads_in_ascii_mode() {
    let at = local_time(RESET).expect("a local time");
    let last_line = STARTED + 31 * MINUTE;
    let building = vec![running(Some("B"), "build", None, STARTED + 26 * MINUTE)];

    let heartbeat = ascii(on_tick(
        &activity(paused(building, 1, None), last_line),
        ms(last_line + 5 * MINUTE),
        Terminal::Plain,
    ));

    assert_eq!(
        heartbeat,
        format!("... still running: B build (10m) | agent launches paused until {at} - 1 waiting")
    );
}

fn text(output: Option<Output>) -> String {
    match output.expect("an output") {
        Output::Footer(text) | Output::Line(text) => text,
    }
}

fn without_spinner(footer: &str) -> &str {
    footer.split_once(' ').expect("a spinner and a text").1
}

#[test]
fn consecutive_redraws_in_the_same_second_change_only_the_spinner() {
    let now = ms(STARTED + 32 * MINUTE + 10);
    let footer = |at: i64| text(on_tick(&activity(busy(), STARTED), at, WIDE));

    let frames: Vec<String> = (0..4).map(|step| footer(now + step * 100)).collect();

    for (frame, footer) in SPINNER.iter().zip(&frames) {
        assert!(footer.starts_with(frame));
    }
    for pair in frames.windows(2) {
        assert_ne!(pair[0], pair[1]);
        assert_eq!(without_spinner(&pair[0]), without_spinner(&pair[1]));
    }
}

#[test]
fn the_times_move_one_second_at_a_time_while_the_spinner_turns_ten_times_a_second() {
    let now = ms(STARTED + 32 * MINUTE + 10);
    let footer = |at: i64| text(on_tick(&activity(busy(), STARTED), at, WIDE));

    assert_eq!(
        without_spinner(&footer(now + 999)),
        without_spinner(&footer(now))
    );
    assert_ne!(
        without_spinner(&footer(now + 50 * MINUTE * 1000)),
        without_spinner(&footer(now))
    );
    assert_eq!(
        without_spinner(&footer(now + 50 * MINUTE * 1000)),
        "── 2 running (B build 56m, C review 51m) · 1 queued · slots 3/3 · 1h22m ──"
    );
    assert_eq!(footer(now), footer(now + 4 * 100 * 3));
}

#[test]
fn the_footer_never_exceeds_the_last_column_even_on_tiny_terminals() {
    let now = ms(STARTED + 32 * MINUTE + 10);

    for width in [0, 1, 2, 5, 30, 80] {
        let terminal = Terminal::Tty { width };
        let limit = width.saturating_sub(1);
        let with = text(on_tick(&activity(busy(), STARTED), now, terminal));
        let without = text(on_tick(&nothing_yet(), now, terminal));

        assert!(with.chars().count() <= limit, "{width}: {with:?}");
        assert!(without.chars().count() <= limit, "{width}: {without:?}");
    }
}

#[test]
fn in_ascii_mode_the_spinner_turns_through_a_bar() {
    let now = ms(STARTED + 32 * MINUTE + 10);
    let footer = |at: i64, activity: &Activity| ascii(on_tick(activity, at, WIDE));

    let frames: Vec<String> = (0..4)
        .map(|step| {
            let footer = footer(now + step * 100, &nothing_yet());
            assert!(footer.is_ascii());
            footer.chars().next().expect("a frame").to_string()
        })
        .collect();

    assert_eq!(frames.join(" "), "| / - \\");
    assert_eq!(footer(now, &nothing_yet()), "| -- 32m --");
    assert!(footer(now + 100, &activity(busy(), STARTED)).starts_with("/ -- 2 running"));
    assert!(footer(now, &activity(busy(), STARTED)).is_ascii());
}

#[test]
fn the_four_spinner_glyphs_are_the_ones_of_the_table() {
    assert_eq!(SPINNER, [SPIN_1, SPIN_2, SPIN_3, SPIN_4]);
}

#[test]
fn without_a_tty_nothing_spins_and_the_heartbeat_stays_one_per_quiet_period() {
    let last_line = STARTED + 31 * MINUTE;
    let quiet = activity(busy(), last_line);
    let heartbeat = "… still running: B build (10m), C review (5m)";

    for offset in [0, 100, 250, 399, 400, 999] {
        let before = on_tick(
            &quiet,
            ms(last_line + 5 * MINUTE) - 1 - offset,
            Terminal::Plain,
        );
        let after = text(on_tick(
            &quiet,
            ms(last_line + 5 * MINUTE) + offset,
            Terminal::Plain,
        ));

        assert_eq!(before, None);
        assert_eq!(after, heartbeat);
        assert!(!after.contains('\r') && !after.contains('\x1b'));
        assert!(!SPINNER.iter().any(|glyph| after.contains(glyph)));
    }
}

#[test]
fn once_a_heartbeat_line_was_printed_the_next_one_waits_a_whole_quiet_period() {
    let printed = STARTED + 36 * MINUTE;
    let quiet = activity(busy(), printed);

    for offset in 0..30 {
        let at = ms(printed) + offset * 100;
        assert_eq!(on_tick(&quiet, at, Terminal::Plain), None);
    }
}
