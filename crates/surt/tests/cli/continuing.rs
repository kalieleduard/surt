use std::fs;

use jiff::Zoned;

use crate::scripts::{crash, spec, tasks};
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const BLOCKED_EXIT_CODE: i32 = 4;
const ACTOR: &str = "tester@example.com";
const SHORT_SHA_CHARS: usize = 6;

fn project() -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox
}

fn feature_name() -> String {
    format!("{}-add-login", Zoned::now().date().strftime("%Y%m%d"))
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    run.stderr
}

fn refused(run: Run) -> String {
    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    run.stderr
}

fn working_copy(sandbox: &Sandbox, rel_path: &str) -> String {
    let path = sandbox
        .dot_surt()
        .join("features")
        .join(feature_name())
        .join(rel_path);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn awaiting(artifact: &str, revision: u32) -> String {
    let name = feature_name();
    format!(
        "⏸ {artifact} rev {revision} awaiting approval in .surt/features/{name}\n  approve: surt approve {name}\n  reject: surt reject {name} -m \"<reason>\"\n"
    )
}

fn identity(sandbox: &Sandbox, artifact: &str, revision: u32) -> String {
    sandbox
        .texts(&format!(
            "SELECT r.identity_sha256 FROM artifact_revision r JOIN artifact a ON a.id = r.artifact_id \
             WHERE a.name = '{artifact}' AND r.number = {revision}"
        ))
        .concat()
}

fn count(sandbox: &Sandbox, table: &str) -> String {
    sandbox
        .texts(&format!("SELECT CAST(count(*) AS TEXT) FROM {table}"))
        .concat()
}

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn continued(sandbox: &Sandbox, name: &str) -> Run {
    surt(sandbox, &["continue", name])
}

fn at_spec_gate(scripts: &[String]) -> Sandbox {
    let sandbox = project();
    sandbox.script(scripts);
    ok(sandbox.run_feature("add login", &[]));
    sandbox
}

#[test]
fn spec_is_rejected_regenerated_approved_then_continued_up_to_the_manifest_gate() {
    let sandbox = project();
    sandbox.script(&[
        spec(1, "# Spec 1\n"),
        spec(2, "# Spec 2 with OAuth\n"),
        tasks(1, &["A", "B"]),
    ]);
    let name = feature_name();

    let created = ok(sandbox.run_feature("add login", &[]));
    let spec_one = working_copy(&sandbox, "spec.md");
    let rejected = ok(surt(
        &sandbox,
        &["reject", "add-login", "-m", "needs OAuth"],
    ));
    let executions_after_reject = count(&sandbox, "execution");
    let resumed = ok(continued(&sandbox, &name));
    let spec_two = working_copy(&sandbox, "spec.md");
    let approved = ok(surt(&sandbox, &["approve", "add-login", "--rev", "2"]));
    let planned = ok(continued(&sandbox, &name));

    assert!(
        created.starts_with(&format!("✔ created {name}")),
        "{created}"
    );
    assert!(created.ends_with(&awaiting("spec", 1)), "{created}");
    assert_eq!(spec_one, "# Spec 1\n");
    assert_eq!(
        rejected,
        format!(
            "✔ rejected spec rev 1 — spec will redo it reading your reason\n  regenerate: surt continue {name}\n"
        )
    );
    assert_eq!(executions_after_reject, "1");
    assert!(resumed.ends_with(&awaiting("spec", 2)), "{resumed}");
    assert_eq!(spec_two, "# Spec 2 with OAuth\n");
    let sha = identity(&sandbox, "spec", 2);
    assert_eq!(
        approved,
        format!(
            "✔ approved spec rev 2 (agent) sha256:{}… by {ACTOR}\n  next: surt continue {name}\n",
            &sha[..SHORT_SHA_CHARS]
        )
    );
    assert!(planned.ends_with(&awaiting("manifest", 1)), "{planned}");
    let manifest: serde_json::Value =
        serde_json::from_str(&working_copy(&sandbox, "manifest.json")).expect("the manifest");
    assert_eq!(manifest.as_array().map(Vec::len), Some(2));
    assert_eq!(working_copy(&sandbox, "tasks/A.md"), "# Task A\n");
    assert_eq!(working_copy(&sandbox, "tasks/B.md"), "# Task B\n");
}

#[test]
fn driving_prints_each_step_as_it_starts_and_finishes() {
    let sandbox = project();
    sandbox.script(&[spec(1, "# Spec 1\n")]);

    let created = ok(sandbox.run_feature("add login", &[]));

    assert!(
        created.contains("\n● spec started\n✔ spec rev 1 published\n✔ spec done\n"),
        "{created}"
    );
}

#[test]
fn continue_while_a_gate_waits_ends_at_that_gate_again_without_running_anything() {
    let sandbox = at_spec_gate(&[spec(1, "# Spec 1\n"), tasks(1, &["A"])]);
    let name = feature_name();

    let waiting = ok(continued(&sandbox, &name));

    assert_eq!(waiting, awaiting("spec", 1));
    assert_eq!(count(&sandbox, "execution"), "1");
}

#[test]
fn a_rejected_manifest_names_the_step_that_redoes_it_and_continue_regenerates_it() {
    let sandbox = at_spec_gate(&[
        spec(1, "# Spec 1\n"),
        tasks(1, &["A", "B", "C"]),
        tasks(2, &["A", "B"]),
    ]);
    let name = feature_name();
    ok(surt(&sandbox, &["approve", &name]));
    ok(continued(&sandbox, &name));

    let rejected = ok(surt(&sandbox, &["reject", &name, "-m", "C is too big"]));
    let replanned = ok(continued(&sandbox, &name));
    let approved = ok(surt(&sandbox, &["approve", &name]));

    assert_eq!(
        rejected,
        format!(
            "✔ rejected manifest rev 1 — plan-tasks will redo it reading your reason\n  regenerate: surt continue {name}\n"
        )
    );
    assert!(replanned.ends_with(&awaiting("manifest", 2)), "{replanned}");
    assert!(
        approved.ends_with(&format!("\n  next: surt continue {name}\n")),
        "{approved}"
    );
}

#[test]
fn a_revision_other_than_the_latest_is_refused() {
    let sandbox = at_spec_gate(&[spec(1, "# Spec 1\n")]);
    let name = feature_name();

    let stale = refused(surt(&sandbox, &["approve", &name, "--rev", "2"]));

    assert!(
        stale.starts_with("✖ spec rev 1 is the latest, not rev 2\n"),
        "{stale}"
    );
    assert_eq!(count(&sandbox, "approval"), "0");
}

#[test]
fn an_ambiguous_prefix_refuses_listing_the_candidates() {
    let sandbox = project();
    sandbox.script(&[spec(1, "# Spec 1\n")]);
    ok(sandbox.run_feature("add login", &[]));
    ok(sandbox.run_feature("add login", &["--new"]));
    let name = feature_name();
    let prefix = &name[..name.len() - 2];

    let ambiguous = refused(surt(&sandbox, &["approve", prefix]));
    let unknown = refused(surt(&sandbox, &["approve", "add-logout"]));

    assert_eq!(
        ambiguous,
        format!(
            "✖ \"{prefix}\" matches 2 features:\n    \
             {name}    AWAITING APPROVAL (spec rev 1)\n    \
             {name}-2  AWAITING APPROVAL (spec rev 1)\n  \
             use the full name\n"
        )
    );
    assert_eq!(
        unknown,
        "✖ no feature matches \"add-logout\"\n  inspect: surt status\n"
    );
    assert_eq!(count(&sandbox, "approval"), "0");
}

#[test]
fn approving_with_no_gate_waiting_is_refused() {
    let sandbox = at_spec_gate(&[spec(1, "# Spec 1\n")]);
    let name = feature_name();
    ok(surt(&sandbox, &["approve", &name]));

    let again = refused(surt(&sandbox, &["approve", &name]));

    assert!(
        again.starts_with(&format!("✖ {name} has no approval gate waiting\n")),
        "{again}"
    );
}

#[test]
fn a_subject_without_an_active_run_is_refused_by_approve_and_reject() {
    let sandbox = at_spec_gate(&[spec(1, "# Spec 1\n")]);
    let name = feature_name();
    sandbox.execute(
        "UPDATE workflow_run SET state = 'cancelled', finished_at = created_at, cleaned_at = created_at",
    );

    for args in [
        vec!["approve", name.as_str()],
        vec!["reject", name.as_str(), "-m", "no"],
    ] {
        let refusal = refused(surt(&sandbox, &args));

        assert_eq!(
            refusal,
            format!("✖ {name} has no active run\n  inspect: surt status {name}\n"),
            "{args:?}"
        );
    }
}

#[test]
fn an_agent_that_keeps_failing_blocks_the_run_with_exit_4() {
    let sandbox = project();
    sandbox.script(&[crash("spec", 1), crash("spec", 2), crash("spec", 3)]);
    let name = feature_name();

    let run = sandbox.run_feature("add login", &[]);

    assert_eq!(run.code, Some(BLOCKED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr.contains("⚠ spec attempt 1 failed (exit_code): "),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr.contains("\n↻ spec attempt 2\n"),
        "{}",
        run.stderr
    );
    let execution = sandbox
        .texts("SELECT id FROM execution WHERE attempt = 3")
        .concat();
    assert!(
        run.stderr.ends_with(&format!(
            "⛔ spec blocked: attempts exhausted\n  attempt 3 failed (exit_code): the agent exited with code 1\n  \
             inspect: surt inspect {execution}\n  logs: surt logs {execution}\n  resume: surt continue {name}\n"
        )),
        "{}",
        run.stderr
    );
}

#[test]
fn an_invalid_runtime_script_is_an_internal_error_before_anything_is_created() {
    let sandbox = project();
    sandbox.script(&["[[on]]\nstep = \"spec\"\nexplode = {}\n".to_owned()]);

    let run = sandbox.run_feature("add login", &[]);

    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(
        run.stderr.contains("invalid Fake Runtime script"),
        "{}",
        run.stderr
    );
    assert_eq!(count(&sandbox, "subject"), "0");
}

#[test]
fn an_edited_spec_is_recorded_by_resuming_and_approved_as_a_human_edit() {
    let sandbox = at_spec_gate(&[spec(1, "# Spec 1\n")]);
    let name = feature_name();
    let spec_file = sandbox
        .dot_surt()
        .join("features")
        .join(&name)
        .join("spec.md");
    fs::write(&spec_file, "# Spec 1, edited\n").expect("the edit is written");

    let resumed = ok(continued(&sandbox, &name));
    fs::remove_file(&spec_file).expect("the spec is deleted");
    let restored = ok(continued(&sandbox, &name));
    let approved = ok(surt(&sandbox, &["approve", &name]));

    assert_eq!(
        resumed,
        format!(
            "✔ your edit to spec was recorded as rev 2\n{}",
            awaiting("spec", 2)
        )
    );
    assert_eq!(
        restored,
        format!(
            "⚠ spec.md was missing; recreated from spec rev 2\n{}",
            awaiting("spec", 2)
        )
    );
    assert!(
        approved.starts_with("✔ approved spec rev 2 (human edit) sha256:"),
        "{approved}"
    );
    assert_eq!(working_copy(&sandbox, "spec.md"), "# Spec 1, edited\n");
}

#[test]
fn an_edit_to_a_pinned_spec_is_refused_until_the_edited_file_is_removed() {
    let sandbox = at_spec_gate(&[spec(1, "# Spec 1\n"), tasks(1, &["A"])]);
    let name = feature_name();
    ok(surt(&sandbox, &["approve", &name]));
    ok(continued(&sandbox, &name));
    let spec_file = sandbox
        .dot_surt()
        .join("features")
        .join(&name)
        .join("spec.md");
    fs::write(&spec_file, "# Spec 1, edited after planning\n").expect("the edit is written");

    let approve = refused(surt(&sandbox, &["approve", &name]));
    let resumed = refused(continued(&sandbox, &name));
    fs::remove_file(&spec_file).expect("the edited spec is removed");
    let approved = ok(surt(&sandbox, &["approve", &name]));

    let refusal = format!(
        "✖ spec.md changed, but spec rev 1 is pinned (a later step already read it)\n  the edit was not recorded\n  undo: surt restore {name} spec\n"
    );
    assert_eq!(approve, refusal);
    assert_eq!(resumed, refusal);
    assert!(
        approved.starts_with(&format!(
            "⚠ .surt/features/{name}/spec.md was missing; recreated from the latest revision\n✔ approved manifest rev 1 (agent)"
        )),
        "{approved}"
    );
    assert_eq!(working_copy(&sandbox, "spec.md"), "# Spec 1\n");
    assert_eq!(
        count(
            &sandbox,
            "artifact_revision WHERE origin = 'human' AND human_action = 'edit'"
        ),
        "0"
    );
}

#[test]
fn a_missing_runtime_script_is_an_internal_error_before_anything_is_created() {
    let sandbox = project();

    let run = sandbox.run_feature("add login", &[]);

    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(
        run.stderr
            .starts_with("✖ internal error: cannot read SURT_RUNTIME="),
        "{}",
        run.stderr
    );
    assert_eq!(count(&sandbox, "subject"), "0");
}
