use std::fs;

use crate::delivering::subject_name;
use crate::scripts::spec;
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const SHORT_SHA_CHARS: usize = 7;

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stderr
}

fn at_the_spec_gate() -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(&[spec(1, "# Spec 1\n")]);
    ok(sandbox.run_feature("add login", &[]));
    sandbox
}

fn cancelled() -> Sandbox {
    let sandbox = at_the_spec_gate();
    ok(surt(&sandbox, &["cancel", "--yes", &subject_name()]));
    sandbox
}

fn roots(sandbox: &Sandbox) -> String {
    sandbox
        .texts("SELECT state FROM workflow_run WHERE parent_step_run_id IS NULL ORDER BY ordinal")
        .join(",")
}

fn head(sandbox: &Sandbox) -> (String, String) {
    let output = |args: &[&str]| sandbox.git_output(&sandbox.repo(), args).trim().to_owned();
    let sha = output(&["rev-parse", "HEAD"]);
    (
        output(&["rev-parse", "--abbrev-ref", "HEAD"]),
        sha[..SHORT_SHA_CHARS].to_owned(),
    )
}

#[test]
fn a_cancelled_subject_starts_over_as_run_two_and_run_one_stays_readable() {
    let name = subject_name();
    let sandbox = cancelled();
    let (branch, short) = head(&sandbox);

    let started = surt(&sandbox, &["continue", &name, "--new-run"]);

    assert_eq!(started.code, Some(0), "{}", started.stderr);
    assert!(
        started.stderr.starts_with(&format!(
            "✔ run 2 of {name}   base: {branch} @ {short}   branch surt/{name}.2\n"
        )),
        "{}",
        started.stderr
    );
    assert!(
        started.stderr.contains("spec rev 1 awaiting approval"),
        "{}",
        started.stderr
    );
    assert_eq!(roots(&sandbox), "cancelled,active");
    assert_eq!(
        sandbox.texts(
            "SELECT w.ordinal || ' ' || r.origin || ' ' || CAST(b.bytes AS TEXT) FROM artifact a \
             JOIN workflow_run w ON w.id = a.workflow_run_id \
             JOIN artifact_revision r ON r.artifact_id = a.id JOIN revision_file f ON f.revision_id = r.id \
             JOIN blob b ON b.sha256 = f.blob_sha256 WHERE a.name = 'request' ORDER BY w.ordinal"
        ),
        ["1 human add login", "2 human add login"]
    );
    let shown = surt(&sandbox, &["status", &name]).stdout;
    assert!(
        shown.starts_with(&format!("{name} · run 2 of 2   AWAITING APPROVAL")),
        "{shown}"
    );
    let first = surt(&sandbox, &["status", &name, "--run", "1"]);
    assert_eq!(first.code, Some(0), "{}", first.stderr);
    assert!(
        first
            .stdout
            .starts_with(&format!("{name} · run 1 of 2   CANCELLED\n")),
        "{}",
        first.stdout
    );
}

#[test]
fn a_subject_without_an_active_run_refuses_and_suggests_a_new_run() {
    let name = subject_name();
    let sandbox = cancelled();

    let refused = surt(&sandbox, &["continue", &name]);

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert!(
        refused
            .stderr
            .starts_with(&format!("✖ {name} has no active run (run 1 cancelled at ")),
        "{}",
        refused.stderr
    );
    assert!(
        refused.stderr.ends_with(&format!(
            ")\n  start run 2 with the same inputs: surt continue {name} --new-run\n"
        )),
        "{}",
        refused.stderr
    );
    assert_eq!(roots(&sandbox), "cancelled");
}

#[test]
fn a_new_run_is_refused_while_the_subject_has_an_active_run() {
    let name = subject_name();
    let sandbox = at_the_spec_gate();

    let refused = surt(&sandbox, &["continue", &name, "--new-run"]);

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert_eq!(
        refused.stderr,
        format!("✖ {name} already has an active run\n  resume: surt continue {name}\n")
    );
    assert_eq!(roots(&sandbox), "active");
}

#[test]
fn a_new_run_is_refused_while_the_working_copy_keeps_an_unrecorded_edit() {
    let name = subject_name();
    let sandbox = at_the_spec_gate();
    let spec = sandbox
        .dot_surt()
        .join("features")
        .join(&name)
        .join("spec.md");
    fs::write(&spec, "# Spec 1, edited\n").expect("the edit is written");
    ok(surt(&sandbox, &["cancel", "--yes", &name]));

    let refused = surt(&sandbox, &["continue", &name, "--new-run"]);

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert_eq!(
        refused.stderr,
        format!(
            "✖ working copy has unrecorded edits from run 1\n    .surt/features/{name}/spec.md\n  \
             fix: move or delete the file, then run again\n"
        )
    );
    assert_eq!(roots(&sandbox), "cancelled");
    fs::remove_file(&spec).expect("the edit is moved away");
    ok(surt(&sandbox, &["continue", &name, "--new-run"]));
    assert_eq!(roots(&sandbox), "cancelled,active");
}

#[test]
fn a_new_run_first_finishes_the_cleanup_of_the_previous_run() {
    let name = subject_name();
    let sandbox = cancelled();
    sandbox.execute("UPDATE workflow_run SET cleaned_at = NULL");

    let started = ok(surt(&sandbox, &["continue", &name, "--new-run"]));

    assert!(
        started.starts_with(&format!(
            "✔ {name} was already cancelled\n↻ cleanup done\n✔ run 2 of {name}   "
        )),
        "{started}"
    );
    assert_eq!(
        sandbox.texts(
            "SELECT CAST(ordinal AS TEXT) FROM workflow_run \
             WHERE parent_step_run_id IS NULL AND cleaned_at IS NOT NULL"
        ),
        ["1"]
    );
}

#[test]
fn a_new_run_of_an_unknown_subject_is_refused() {
    let sandbox = at_the_spec_gate();

    let refused = surt(&sandbox, &["continue", "20200101-nothing", "--new-run"]);

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert!(
        refused.stderr.starts_with("✖ no subject matches"),
        "{}",
        refused.stderr
    );
}
