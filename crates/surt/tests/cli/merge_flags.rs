use crate::delivering::subject_name;
use crate::scripts::spec;
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const USAGE_EXIT_CODE: i32 = 2;

fn project() -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(&[spec(1, "# Spec 1\n")]);
    sandbox
}

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stderr
}

fn second_line(output: &str) -> &str {
    output.lines().nth(1).unwrap_or_default()
}

fn frozen_mode(sandbox: &Sandbox, name: &str) -> String {
    let shown = surt(sandbox, &["status", name, "--json"]).stdout;
    let status: serde_json::Value = serde_json::from_str(&shown).expect("status is JSON");
    status["merge_into_base"]
        .as_str()
        .unwrap_or_else(|| panic!("{shown}"))
        .to_owned()
}

fn cancelled() -> Sandbox {
    let sandbox = project();
    ok(sandbox.run_feature("add login", &[]));
    ok(surt(&sandbox, &["cancel", "--yes", &subject_name()]));
    sandbox
}

#[test]
fn merge_freezes_ff_only_and_names_the_flag_under_the_creation_line() {
    let sandbox = project();

    let created = ok(sandbox.run_feature("add login", &["--merge"]));

    assert!(created.starts_with("✔ created "), "{created}");
    assert_eq!(
        second_line(&created),
        "  merge into base: ff-only (--merge)",
        "{created}"
    );
    assert_eq!(frozen_mode(&sandbox, &subject_name()), "ff-only");
}

#[test]
fn no_merge_freezes_never_over_a_project_file_that_says_ff_only() {
    let sandbox = project();
    sandbox.write_config("[git]\nmerge_into_base = \"ff-only\"\n");

    let created = ok(sandbox.run_feature("add login", &["--no-merge"]));

    assert_eq!(
        second_line(&created),
        "  merge into base: never (--no-merge)",
        "{created}"
    );
    assert_eq!(frozen_mode(&sandbox, &subject_name()), "never");
}

#[test]
fn a_project_file_that_says_ff_only_is_named_without_a_flag() {
    let sandbox = project();
    sandbox.write_config("[git]\nmerge_into_base = \"ff-only\"\n");

    let created = ok(sandbox.run_feature("add login", &[]));

    assert_eq!(
        second_line(&created),
        "  merge into base: ff-only",
        "{created}"
    );
}

#[test]
fn the_default_never_without_a_flag_prints_nothing_extra() {
    let sandbox = project();

    let created = ok(sandbox.run_feature("add login", &[]));

    assert!(!created.contains("merge into base"), "{created}");
}

#[test]
fn merge_and_no_merge_together_are_a_usage_error() {
    let sandbox = project();

    let both = sandbox.run_feature("add login", &["--merge", "--no-merge"]);

    assert_eq!(both.code, Some(USAGE_EXIT_CODE), "{}", both.stderr);
    assert!(sandbox.texts("SELECT id FROM workflow_run").is_empty());
}

#[test]
fn a_new_run_freezes_the_flag_it_was_given() {
    let name = subject_name();
    let sandbox = cancelled();

    let merged = ok(surt(&sandbox, &["continue", &name, "--new-run", "--merge"]));

    assert!(merged.starts_with("✔ run 2 of "), "{merged}");
    assert_eq!(
        second_line(&merged),
        "  merge into base: ff-only (--merge)",
        "{merged}"
    );
    assert_eq!(frozen_mode(&sandbox, &name), "ff-only");
}

#[test]
fn a_new_run_freezes_no_merge_over_the_project_file() {
    let name = subject_name();
    let sandbox = cancelled();
    sandbox.write_config("[git]\nmerge_into_base = \"ff-only\"\n");

    let kept_off = ok(surt(
        &sandbox,
        &["continue", &name, "--new-run", "--no-merge"],
    ));

    assert_eq!(
        second_line(&kept_off),
        "  merge into base: never (--no-merge)",
        "{kept_off}"
    );
    assert_eq!(frozen_mode(&sandbox, &name), "never");
}

#[test]
fn either_flag_on_resume_refuses_naming_the_frozen_value_and_the_new_run() {
    let name = subject_name();
    let sandbox = project();
    ok(sandbox.run_feature("add login", &["--merge"]));

    for flag in ["--merge", "--no-merge"] {
        let resumed = sandbox.continue_run(&[&name, flag]);

        assert_eq!(resumed.code, Some(REFUSED_EXIT_CODE), "{}", resumed.stderr);
        assert_eq!(
            resumed.stderr,
            format!(
                "✖ run 1 of {name} already froze merge_into_base = \"ff-only\"\n  \
                 to choose again, start a new run: surt continue {name} --new-run {flag}\n"
            )
        );
    }
    assert_eq!(frozen_mode(&sandbox, &name), "ff-only");
}

#[test]
fn either_flag_on_a_subject_without_an_active_run_refuses_naming_the_latest_run() {
    let name = subject_name();
    let sandbox = cancelled();

    let resumed = sandbox.continue_run(&[&name, "--merge"]);

    assert_eq!(resumed.code, Some(REFUSED_EXIT_CODE), "{}", resumed.stderr);
    assert_eq!(
        resumed.stderr,
        format!(
            "✖ run 1 of {name} already froze merge_into_base = \"never\"\n  \
             to choose again, start a new run: surt continue {name} --new-run --merge\n"
        )
    );
}
