use crate::scripts::tasks;
use crate::start::{feature_name, planned};
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

#[cfg(unix)]
use crate::interrupt::{Driving, waiting_builder};

#[cfg(unix)]
const CANCELLED_EXIT_CODE: i32 = 5;

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn root_state(sandbox: &Sandbox) -> String {
    sandbox
        .texts("SELECT state FROM workflow_run WHERE parent_step_run_id IS NULL ORDER BY rowid")
        .join(",")
}

#[test]
fn cancel_without_a_terminal_and_without_yes_refuses_and_changes_nothing() {
    let name = feature_name();
    let sandbox = planned(&name, &[tasks(1, &["A"])]);

    let refused = surt(&sandbox, &["cancel", &name]);

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert_eq!(
        refused.stderr,
        format!(
            "✖ cancel asks for a confirmation and there is no terminal to ask\n  confirm: surt cancel {name} --yes\n"
        )
    );
    assert_eq!(root_state(&sandbox), "active");
}

#[test]
fn cancel_of_a_feature_without_an_active_run_is_refused() {
    let name = feature_name();
    let sandbox = planned(&name, &[tasks(1, &["A"])]);
    let first = surt(&sandbox, &["cancel", "--yes", &name]);
    assert_eq!(first.code, Some(0), "{}", first.stderr);

    let again = surt(&sandbox, &["cancel", "--yes", &name]);

    assert_eq!(again.code, Some(REFUSED_EXIT_CODE), "{}", again.stderr);
    assert_eq!(
        again.stderr,
        format!("✖ {name} has no active run\n  inspect: surt status {name}\n")
    );
}

#[cfg(unix)]
#[test]
fn cancel_of_a_crashed_run_settles_it_and_a_new_description_starts_a_second_feature() {
    let name = feature_name();
    let sandbox = waiting_builder(&name);
    let mut driving = Driving::start(&sandbox, &name);
    driving.wait_for("● task A build started\n");
    driving.signal("KILL");
    let (killed, _) = driving.finish();
    assert_eq!(killed, None);

    let cancelled = surt(&sandbox, &["cancel", "--yes", &name]);

    assert_eq!(cancelled.code, Some(0), "{}", cancelled.stderr);
    assert!(
        cancelled.stderr.starts_with(
            "✖ task A build attempt 1 exited but was never recorded — discarded\n✔ run cancelled\n↻ cleanup 1 worktree · working copy\n"
        ),
        "{}",
        cancelled.stderr
    );
    assert_eq!(root_state(&sandbox), "cancelled");
    let started = sandbox.feature(&["add login"]);
    assert_eq!(started.code, Some(0), "{}", started.stderr);
    assert!(
        started.stderr.contains(&format!("✔ created {name}-2 ")),
        "{}",
        started.stderr
    );
    assert_eq!(root_state(&sandbox), "cancelled,active");
}

#[cfg(unix)]
#[test]
fn cancel_from_another_terminal_makes_the_driver_stop_and_exit_5() {
    let name = feature_name();
    let sandbox = waiting_builder(&name);
    let mut driving = Driving::start(&sandbox, &name);
    driving.wait_for("● task A build started\n");
    let driver = driving.pid();

    let cancelled = surt(&sandbox, &["cancel", "--yes", &name]);
    let (code, stderr) = driving.finish();

    assert_eq!(cancelled.code, Some(0), "{}", cancelled.stderr);
    assert!(
        cancelled.stderr.starts_with(&format!(
            "⏳ cancel requested; driver pid {driver} is stopping it…\n✔ run cancelled\n"
        )),
        "{}",
        cancelled.stderr
    );
    assert_eq!(code, Some(CANCELLED_EXIT_CODE), "{stderr}");
    assert!(
        stderr.ends_with(&format!(
            "✖ cancelled by surt cancel (from another terminal)\n  inspect: surt status {name}\n"
        )),
        "{stderr}"
    );
    assert_eq!(root_state(&sandbox), "cancelled");
    assert_eq!(
        sandbox
            .texts("SELECT state FROM execution ORDER BY rowid DESC LIMIT 1")
            .concat(),
        "cancelled"
    );
}
