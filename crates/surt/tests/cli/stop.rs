use crate::delivering::{planned, subject_name};
use crate::scripts::tasks;
use crate::support::{REFUSED_EXIT_CODE, Sandbox};

#[cfg(unix)]
use crate::interrupt::Driving;
#[cfg(unix)]
use crate::scripts::{build, review, ship, wait_until_stopped};

#[cfg(unix)]
const INTERRUPTED_EXIT_CODE: i32 = 130;

fn stop_requested(sandbox: &Sandbox) -> Vec<String> {
    sandbox.texts(
        "SELECT coalesce(stop_requested_by, 'none') FROM workflow_run \
         WHERE parent_step_run_id IS NULL",
    )
}

#[test]
fn stop_without_a_live_driver_refuses_with_the_run_state() {
    let name = subject_name();
    let sandbox = planned(&name, &[tasks(1, &["A"])]);

    let refused = sandbox.surt(&sandbox.repo(), &["stop", &name]);

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert_eq!(
        refused.stderr,
        format!(
            "✖ {name} has no live driver, nothing to stop (INTERRUPTED)\n  inspect: surt status {name}\n"
        )
    );
    assert_eq!(stop_requested(&sandbox), ["none"]);
}

#[cfg(unix)]
#[test]
fn stop_from_another_terminal_ends_the_driver_with_130_and_start_resumes_the_run() {
    let name = subject_name();
    let sandbox = planned(
        &name,
        &[
            tasks(1, &["A"]),
            wait_until_stopped("A/build"),
            build("A", 1, 2, "login.txt", "login\n"),
            review("A", 1, "approved"),
            ship(),
        ],
    );
    let mut driving = Driving::start(&sandbox, &name);
    driving.wait_for("● item A build started\n");
    let driver = driving.pid();

    let stopped = sandbox.surt(&sandbox.repo(), &["stop", &name]);
    let (code, stderr) = driving.finish();

    assert_eq!(stopped.code, Some(0), "{}", stopped.stderr);
    assert_eq!(
        stopped.stderr,
        format!(
            "⏳ stop requested; driver pid {driver} is stopping 1 execution…\n\
             ✔ stopped. resumable: surt continue {name}\n"
        )
    );
    assert_eq!(code, Some(INTERRUPTED_EXIT_CODE), "{stderr}");
    assert!(
        stderr.contains(
            "⏳ stop requested from another terminal; stopping…\n\
             ⏹ item A build attempt 1 interrupted\n"
        ),
        "{stderr}"
    );
    assert!(
        stderr.ends_with(&format!(
            "⏹ stopped by surt stop (from another terminal)\n  resume: surt continue {name}\n"
        )),
        "{stderr}"
    );
    assert_ne!(stop_requested(&sandbox), ["none"]);
    let timeline = sandbox.surt(&sandbox.repo(), &["inspect", &name]).stdout;
    assert!(
        timeline
            .lines()
            .any(|line| line.contains("  run.stop_requested  ")
                && line.ends_with("  by tester@example.com")),
        "{timeline}"
    );

    let resumed = sandbox.surt(&sandbox.repo(), &["continue", &name]);

    assert_eq!(resumed.code, Some(0), "{}", resumed.stderr);
    assert!(
        resumed.stderr.contains("↻ item A build attempt 2\n"),
        "{}",
        resumed.stderr
    );
    assert!(
        resumed.stderr.contains("✔ run succeeded\n"),
        "{}",
        resumed.stderr
    );
    assert_eq!(stop_requested(&sandbox), ["none"]);
}
