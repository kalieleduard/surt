use std::path::PathBuf;

use surt_engine::project::{ActivityPause, Progress, StepLabel};

use super::message;
use crate::driving::blocked::local_time;
use crate::symbols::Symbols;

fn line(progress: &Progress) -> String {
    let (mark, message) = message(progress);
    message.plain(mark)
}

fn task(item: &str, key: &str) -> StepLabel {
    StepLabel {
        key: key.to_owned(),
        item: Some(item.to_owned()),
        iteration: Some(1),
    }
}

#[test]
fn recovery_reports_what_it_found_line_by_line() {
    let found = [
        Progress::Recovering {
            driver: Some(48213),
        },
        Progress::OrphanStopped {
            step: task("B", "build"),
            attempt: 1,
            pid: 50122,
        },
        Progress::Discarded {
            step: task("C", "build"),
            attempt: 1,
        },
        Progress::InterruptedWithDriver {
            step: task("D", "build"),
            attempt: 2,
        },
        Progress::IntegrationRecorded {
            step: task("A", "integrate"),
            branch: "surt/20261017-add-login".to_owned(),
            merge: "3c1aa20f6b4d2e1c0a9b8c7d6e5f4a3b2c1d0e9f".to_owned(),
        },
        Progress::Recovering { driver: None },
    ];

    let lines: String = found.iter().map(line).collect();

    assert_eq!(
        lines,
        "⚠ previous driver (pid 48213) is gone — recovering\n\
         ⚠ task B build attempt 1 (pid 50122, alive) — stopped\n\
         ✖ task C build attempt 1 exited but was never recorded — discarded\n\
         ⚠ task D build attempt 2 was interrupted with the previous driver — discarded\n\
         ✔ task A integrate had completed (surt/20261017-add-login @ 3c1aa20) — recorded\n\
         ⚠ previous driver is gone — recovering\n"
    );
}

#[test]
fn a_cancel_request_says_which_driver_is_stopping_the_run() {
    let requested = [
        Progress::CancelRequested {
            driver: Some(48213),
        },
        Progress::CancelRequested { driver: None },
    ];

    let lines: String = requested.iter().map(line).collect();

    assert_eq!(
        lines,
        "⏳ cancel requested; driver pid 48213 is stopping it…\n\
         ⏳ cancel requested; waiting for the driver to stop…\n"
    );
}

#[test]
fn recovery_and_cancel_lines_print_only_ascii_in_ascii_mode() {
    let found = [
        Progress::Recovering {
            driver: Some(48213),
        },
        Progress::Discarded {
            step: task("C", "build"),
            attempt: 1,
        },
        Progress::CancelRequested { driver: None },
    ];

    let lines: String = found.iter().map(line).collect();

    assert_eq!(
        Symbols::Ascii.render(&lines),
        "[warn] previous driver (pid 48213) is gone - recovering\n\
         [error] task C build attempt 1 exited but was never recorded - discarded\n\
         [waiting] cancel requested; waiting for the driver to stop...\n"
    );
}

#[test]
fn an_invalid_configuration_mid_run_says_which_limit_the_driver_keeps() {
    let warning = Progress::ConfigInvalid {
        path: PathBuf::from("/work/shop/.surt/config.toml"),
        line: Some(4),
        reason: "string values must be quoted".to_owned(),
        limit: 2,
    };

    let printed = line(&warning);

    assert_eq!(
        printed,
        "⚠ invalid configuration in /work/shop/.surt/config.toml:4 — keeping max_concurrent_agents = 2\n\
         \x20 string values must be quoted\n\
         \x20 fix: edit /work/shop/.surt/config.toml:4\n"
    );
    assert_eq!(
        Symbols::Ascii.render(&printed),
        "[warn] invalid configuration in /work/shop/.surt/config.toml:4 - keeping max_concurrent_agents = 2\n\
         \x20 string values must be quoted\n\
         \x20 fix: edit /work/shop/.surt/config.toml:4\n"
    );
}

#[test]
fn stop_and_salvage_lines_print_only_ascii_in_ascii_mode() {
    let lines: String = [
        Progress::StopRequested,
        Progress::AttemptInterrupted {
            step: task("B", "build"),
            attempt: 1,
        },
        Progress::Salvaged {
            step: task("B", "build"),
            reference: "refs/surt/salvage/0192f3a8".to_owned(),
        },
    ]
    .iter()
    .map(line)
    .collect();

    assert_eq!(
        Symbols::Ascii.render(&lines),
        "[waiting] stop requested from another terminal; stopping...\n\
         [stopped] task B build attempt 1 interrupted\n\
         [warn] task B build worktree had leftovers - saved to refs/surt/salvage/0192f3a8\n\
         \x20 inspect: git log -p refs/surt/salvage/0192f3a8\n"
    );
}

const RESET: i64 = 1_792_900_800;

#[test]
fn launches_pause_until_the_reset_time_or_for_the_backoff_delay_then_resume() {
    let at = local_time(RESET).expect("a local time");
    let lines: String = [
        Progress::Paused {
            pause: ActivityPause {
                until: RESET,
                backoff: None,
            },
            waiting: 2,
        },
        Progress::Paused {
            pause: ActivityPause {
                until: RESET,
                backoff: Some(120),
            },
            waiting: 1,
        },
        Progress::Resumed {
            paused_seconds: 2 * 3600 + 5 * 60,
        },
    ]
    .iter()
    .map(line)
    .collect();

    assert_eq!(
        lines,
        format!(
            "⏸ agent launches paused until {at} — 2 waiting · Ctrl-C to stop (resumable)\n\
             ⏸ agent launches paused until {at} (backoff 2m) — 1 waiting · Ctrl-C to stop (resumable)\n\
             ↻ agent launches resumed after 2h05m\n"
        )
    );
    assert_eq!(
        Symbols::Ascii.render(&lines),
        format!(
            "[paused] agent launches paused until {at} - 2 waiting | Ctrl-C to stop (resumable)\n\
             [paused] agent launches paused until {at} (backoff 2m) - 1 waiting | Ctrl-C to stop (resumable)\n\
             [retry] agent launches resumed after 2h05m\n"
        )
    );
}
