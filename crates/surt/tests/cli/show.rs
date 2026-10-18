use std::fs;

use crate::scripts::{spec, tasks};
use crate::start::feature_name;
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const SPEC_1: &str = "# Spec 1\n";
const SPEC_2: &str = "# Spec 2\nwith OAuth\n";

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn ok(run: Run) -> Run {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run
}

fn refused(run: Run) -> String {
    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    run.stderr
}

fn with_two_specs(name: &str) -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(&[spec(1, SPEC_1), spec(2, SPEC_2), tasks(1, &["A", "B"])]);
    ok(sandbox.feature(&["add login"]));
    ok(surt(
        &sandbox,
        &["reject", name, "-m", "falta o fluxo OAuth"],
    ));
    ok(surt(&sandbox, &["feature", name]));
    ok(surt(&sandbox, &["approve", name]));
    ok(surt(&sandbox, &["plan", name]));
    sandbox
}

#[test]
fn show_lists_the_artifacts_of_a_feature_with_their_latest_revision() {
    let name = feature_name();
    let sandbox = with_two_specs(&name);

    let run = ok(surt(&sandbox, &["show", "add-login"]));

    assert_eq!(
        run.stdout,
        "ARTIFACT               REVS  LATEST\n\
         request                1     rev 1 (human)\n\
         base                   1     rev 1 (human)\n\
         spec                   2     rev 2 (agent) ✔ approved\n\
         spec-rejection-reason  1     rev 1 (human)\n\
         manifest               1     rev 1 (agent) · 3 files\n"
    );
}

#[test]
fn show_prints_any_revision_byte_for_byte_on_stdout_and_nothing_else() {
    let name = feature_name();
    let sandbox = with_two_specs(&name);

    let latest = ok(surt(&sandbox, &["show", &name, "spec"]));
    let first = ok(surt(&sandbox, &["show", &name, "spec", "--rev", "1"]));
    let manifest = ok(surt(&sandbox, &["show", &name, "manifest"]));

    assert_eq!(latest.stdout, SPEC_2);
    assert_eq!(first.stdout, SPEC_1);
    assert!(latest.stderr.is_empty(), "{}", latest.stderr);
    let working_copy = sandbox.dot_surt().join("features").join(&name);
    assert_eq!(
        manifest.stdout,
        fs::read_to_string(working_copy.join("manifest.json")).expect("the manifest")
    );
}

#[test]
fn show_warns_on_stderr_when_the_working_copy_holds_an_unrecorded_edit() {
    let name = feature_name();
    let sandbox = with_two_specs(&name);
    let working_copy = sandbox.dot_surt().join("features").join(&name);
    fs::write(working_copy.join("spec.md"), "# Spec, edited by hand\n").expect("an edit");

    let run = ok(surt(&sandbox, &["show", &name, "spec"]));

    assert_eq!(run.stdout, SPEC_2);
    assert_eq!(
        run.stderr,
        "⚠ Working Copy has an unrecorded edit to spec.md\n  this is the recorded spec rev 2\n"
    );
}

#[test]
fn show_refuses_an_unknown_artifact_listing_the_existing_ones() {
    let name = feature_name();
    let sandbox = with_two_specs(&name);

    let unknown = refused(surt(&sandbox, &["show", &name, "specs"]));
    let missing_revision = refused(surt(&sandbox, &["show", &name, "spec", "--rev", "3"]));
    let unknown_task = refused(surt(&sandbox, &["show", &name, "feedback", "--task", "A"]));

    assert_eq!(
        unknown,
        format!(
            "✖ {name} has no artifact \"specs\"\n  \
             artifacts: request, base, spec, spec-rejection-reason, manifest\n  \
             inspect: surt show {name}\n"
        )
    );
    assert_eq!(
        missing_revision,
        format!(
            "✖ spec has no rev 3; the latest is rev 2\n  \
             inspect: surt show {name} spec --rev 2\n"
        )
    );
    assert_eq!(
        unknown_task,
        format!(
            "✖ {name} has no task \"A\"\n  \
             it has no tasks yet\n  \
             inspect: surt status {name}\n"
        )
    );
}
