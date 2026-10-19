use crate::delivering::subject_name;
use crate::scripts::spec;
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const USAGE_EXIT_CODE: i32 = 2;
const SHORT_SHA_CHARS: usize = 7;
const CHANGED_REQUEST: &str = "add login with sso";

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stderr
}

fn cancelled() -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(&[spec(1, "# Spec 1\n")]);
    ok(sandbox.run_feature("add login", &[]));
    ok(surt(&sandbox, &["cancel", "--yes", &subject_name()]));
    sandbox
}

fn head(sandbox: &Sandbox) -> String {
    sandbox
        .git_output(&sandbox.repo(), &["rev-parse", "HEAD"])
        .trim()
        .to_owned()
}

fn input_of_run(sandbox: &Sandbox, input: &str, run: u32) -> String {
    sandbox
        .texts(&format!(
            "SELECT CAST(b.bytes AS TEXT) FROM artifact a \
             JOIN workflow_run w ON w.id = a.workflow_run_id \
             JOIN artifact_revision r ON r.artifact_id = a.id JOIN revision_file f ON f.revision_id = r.id \
             JOIN blob b ON b.sha256 = f.blob_sha256 \
             WHERE a.name = '{input}' AND w.ordinal = {run} AND w.parent_step_run_id IS NULL"
        ))
        .concat()
}

fn root_runs(sandbox: &Sandbox) -> String {
    sandbox
        .texts("SELECT CAST(count(*) AS TEXT) FROM workflow_run WHERE parent_step_run_id IS NULL")
        .concat()
}

#[test]
fn a_new_run_carries_the_request_and_resolves_the_base_again() {
    let name = subject_name();
    let sandbox = cancelled();
    let first_base = head(&sandbox);
    sandbox.git(
        &sandbox.repo(),
        &["commit", "--quiet", "--allow-empty", "-m", "moved on"],
    );
    let moved = head(&sandbox);

    let started = ok(surt(&sandbox, &["continue", &name, "--new-run"]));

    assert!(
        started.starts_with(&format!(
            "✔ run 2 of {name}   base: master @ {}   branch surt/{name}.2\n",
            &moved[..SHORT_SHA_CHARS]
        )),
        "{started}"
    );
    assert_eq!(input_of_run(&sandbox, "request", 2), "add login");
    assert!(input_of_run(&sandbox, "base", 1).contains(&first_base));
    assert!(input_of_run(&sandbox, "base", 2).contains(&moved));
}

#[test]
fn an_input_given_to_a_new_run_replaces_the_carried_one() {
    let name = subject_name();
    let sandbox = cancelled();
    let request = format!("request={CHANGED_REQUEST}");

    ok(surt(
        &sandbox,
        &["continue", &name, "--new-run", "--input", &request],
    ));

    assert_eq!(input_of_run(&sandbox, "request", 1), "add login");
    assert_eq!(input_of_run(&sandbox, "request", 2), CHANGED_REQUEST);
}

#[test]
fn an_input_the_workflow_does_not_declare_refuses_the_new_run() {
    let name = subject_name();
    let sandbox = cancelled();

    let refused = surt(
        &sandbox,
        &["continue", &name, "--new-run", "--input", "colour=blue"],
    );

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert!(
        refused.stderr.contains("inputs of feature:"),
        "{}",
        refused.stderr
    );
    assert_eq!(root_runs(&sandbox), "1");
}

#[test]
fn an_input_without_new_run_is_a_usage_error() {
    let name = subject_name();
    let sandbox = cancelled();
    let request = format!("request={CHANGED_REQUEST}");

    let refused = surt(&sandbox, &["continue", &name, "--input", &request]);

    assert_eq!(refused.code, Some(USAGE_EXIT_CODE), "{}", refused.stderr);
    assert!(refused.stderr.contains("--new-run"), "{}", refused.stderr);
    assert_eq!(root_runs(&sandbox), "1");
}
