use surt_engine::project::{Progress, StepLabel};

use super::message;

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
