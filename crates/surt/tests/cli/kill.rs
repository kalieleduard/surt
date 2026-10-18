use std::io::{BufRead, BufReader};

use crate::scripts::{build, review, ship, tasks, wait_until_stopped};
use crate::start::{feature_name, planned};
use crate::support::Sandbox;

const BUILD_STARTED: &str = "● task A build started\n";

fn killed_while_building(sandbox: &Sandbox, name: &str) -> u32 {
    let mut child = sandbox.spawn(&["start", name]);
    let mut stderr = BufReader::new(child.stderr.take().expect("the piped stderr"));
    let mut seen = String::new();
    while !seen.ends_with(BUILD_STARTED) {
        let read = stderr.read_line(&mut seen).expect("stderr is readable");
        assert_ne!(read, 0, "stderr ended before the build started:\n{seen}");
    }
    let pid = child.id();
    child.kill().expect("the driver is killed");
    let status = child.wait().expect("the killed driver is reaped");
    assert!(!status.success(), "{status:?}");
    pid
}

#[test]
fn a_killed_driver_frees_the_run_and_the_next_start_finishes_it_with_a_new_attempt() {
    let name = feature_name();
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
    let killed = killed_while_building(&sandbox, &name);
    assert_eq!(
        sandbox.texts(
            "SELECT e.state FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             WHERE s.step_key = 'build'"
        ),
        ["running"]
    );

    let resumed = sandbox.surt(&sandbox.repo(), &["start", &name]);

    assert_eq!(resumed.code, Some(0), "{}", resumed.stderr);
    for line in [
        format!("previous driver (pid {killed}) is gone — recovering\n"),
        "task A build attempt 1 exited but was never recorded — discarded\n".to_owned(),
        "↻ task A build attempt 2\n".to_owned(),
        "✔ run succeeded\n".to_owned(),
    ] {
        assert!(resumed.stderr.contains(&line), "{line}\n{}", resumed.stderr);
    }
    assert_eq!(
        sandbox.texts(
            "SELECT e.attempt || ' ' || e.state FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id WHERE s.step_key = 'build' ORDER BY e.attempt"
        ),
        ["1 interrupted", "2 succeeded"]
    );
    assert_eq!(
        sandbox.texts("SELECT state FROM workflow_run WHERE parent_step_run_id IS NULL"),
        ["succeeded"]
    );
}
