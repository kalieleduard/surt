use std::fs;

use crate::scripts::{build, review, ship, spec, tasks};
use crate::start::feature_name;
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const TEMPLATE_MODEL: &str = "model = \"claude-sonnet-5-5\"";
const EDITED_MODEL: &str = "claude-opus-5-5";

fn project(scripts: &[String]) -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(scripts);
    sandbox
}

fn edit(sandbox: &Sandbox, rel_path: &str, from: &str, to: &str) {
    let path = sandbox.dot_surt().join(rel_path);
    let text = fs::read_to_string(&path).expect("the template file");
    assert!(text.contains(from), "{from} not in {rel_path}");
    fs::write(path, text.replace(from, to)).expect("the edited file");
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stderr
}

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn assert_nothing_written(sandbox: &Sandbox) {
    assert_eq!(
        sandbox.texts("SELECT CAST(count(*) AS TEXT) FROM subject"),
        ["0"]
    );
}

#[test]
fn the_creation_line_stands_alone_without_a_local_agents_line() {
    let sandbox = project(&[spec(1, "# Spec 1\n")]);

    let created = ok(sandbox.feature(&["add login"]));

    assert!(created.starts_with("✔ created "), "{created}");
    assert!(!created.contains("local agents"), "{created}");
}

#[test]
fn an_invalid_agent_file_refuses_the_feature_with_exit_3_before_any_row_is_written() {
    let sandbox = project(&[spec(1, "# Spec 1\n")]);
    edit(
        &sandbox,
        "agents/builder/agent.toml",
        "effort = \"high\"",
        "effort = \"huge\"",
    );

    let run = sandbox.feature(&["add login"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr.starts_with(
            "✖ .surt/agents/builder/agent.toml: profile.effort: unknown variant `huge`"
        ),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr
            .ends_with("\n  fix: edit .surt/agents/builder/agent.toml\n"),
        "{}",
        run.stderr
    );
    assert_nothing_written(&sandbox);
}

#[test]
fn removing_the_outcome_a_declared_check_names_is_refused_at_outcomes() {
    let sandbox = project(&[spec(1, "# Spec 1\n")]);
    edit(
        &sandbox,
        "agents/reviewer/agent.toml",
        "outcomes = [\"approved\", \"changes_requested\"]",
        "outcomes = [\"changes_requested\"]",
    );

    let run = sandbox.feature(&["add login"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "✖ .surt/agents/reviewer/agent.toml: outcomes: a delivery check needs the outcome \"approved\"\n  \
         fix: edit .surt/agents/reviewer/agent.toml\n"
    );
    assert_nothing_written(&sandbox);
}

#[test]
fn without_the_feature_workflow_the_feature_is_refused_listing_the_workflows() {
    let sandbox = project(&[spec(1, "# Spec 1\n")]);
    fs::remove_file(sandbox.dot_surt().join("workflows/feature.toml")).expect("removed");

    let run = sandbox.feature(&["add login"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "✖ no workflow is named \"feature\" in .surt/workflows/\n  workflows: task-cycle\n  \
         fix: add .surt/workflows/feature.toml\n"
    );
    assert_nothing_written(&sandbox);
}

#[test]
fn a_project_initialized_without_a_template_is_pointed_at_it() {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    ok(surt(&sandbox, &["init"]));
    sandbox.script(&[spec(1, "# Spec 1\n")]);

    let run = sandbox.feature(&["add login"]);

    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr
            .ends_with("\n  fix: surt init --template feature\n"),
        "{}",
        run.stderr
    );
}

#[test]
fn plan_and_start_drive_a_feature_whose_builder_model_was_edited() {
    let sandbox = project(&[
        spec(1, "# Spec 1\n"),
        tasks(1, &["A"]),
        build("A", 1, 1, "login.txt", "login\n"),
        review("A", 1, "approved"),
        ship(),
    ]);
    edit(
        &sandbox,
        "agents/builder/agent.toml",
        TEMPLATE_MODEL,
        &format!("model = \"{EDITED_MODEL}\""),
    );
    let name = feature_name();

    ok(sandbox.feature(&["add login"]));
    ok(surt(&sandbox, &["approve", &name]));
    ok(surt(&sandbox, &["plan", &name]));
    ok(surt(&sandbox, &["approve", &name]));
    let started = ok(surt(&sandbox, &["start", &name]));

    assert!(started.contains("✔ run succeeded\n"), "{started}");
    assert_eq!(
        sandbox.texts(
            "SELECT json_extract(e.runtime_info, '$.model') FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id WHERE s.step_key = 'build'"
        ),
        [EDITED_MODEL]
    );
}

#[test]
fn editing_an_agent_after_creation_leaves_the_run_on_the_frozen_definition() {
    let sandbox = project(&[
        spec(1, "# Spec 1\n"),
        tasks(1, &["A"]),
        build("A", 1, 1, "login.txt", "login\n"),
        review("A", 1, "approved"),
        ship(),
    ]);
    let name = feature_name();

    ok(sandbox.feature(&["add login"]));
    edit(
        &sandbox,
        "agents/builder/agent.toml",
        TEMPLATE_MODEL,
        &format!("model = \"{EDITED_MODEL}\""),
    );
    ok(surt(&sandbox, &["approve", &name]));
    ok(surt(&sandbox, &["plan", &name]));
    ok(surt(&sandbox, &["approve", &name]));
    ok(surt(&sandbox, &["start", &name]));

    assert_eq!(
        sandbox.texts(
            "SELECT json_extract(e.runtime_info, '$.model') FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id WHERE s.step_key = 'build'"
        ),
        ["claude-sonnet-5-5"]
    );
}
