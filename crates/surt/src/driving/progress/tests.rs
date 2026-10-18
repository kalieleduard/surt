use surt_engine::project::{Progress, StepLabel};

use super::message;
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
