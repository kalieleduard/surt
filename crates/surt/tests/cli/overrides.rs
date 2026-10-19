use std::fs;
use std::path::Path;

use crate::scripts::{build, review, ship, spec, tasks};
use crate::start::feature_name;
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const LOCAL_MODEL: &str = "claude-opus-5-5";

fn project(scripts: &[String]) -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(scripts);
    sandbox
}

fn write_local(sandbox: &Sandbox, agent: &str, text: &str) {
    write_local_file(sandbox, agent, "agent.toml", text);
}

fn write_local_file(sandbox: &Sandbox, agent: &str, rel_path: &str, text: &str) {
    let path = sandbox.dot_surt().join("agents").join(agent).join(rel_path);
    fs::create_dir_all(path.parent().expect("a parent")).expect("the agent directory");
    fs::write(path, text).expect("the local file");
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stderr
}

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

#[test]
fn the_creation_line_lists_the_agents_that_use_local_files() {
    let sandbox = project(&[spec(1, "# Spec 1\n")]);
    write_local(
        &sandbox,
        "spec",
        &format!("[profile]\nmodel = \"{LOCAL_MODEL}\"\n"),
    );

    let created = ok(sandbox.feature(&["add login"]));

    let lines: Vec<&str> = created.lines().take(2).collect();
    assert!(lines[0].starts_with("✔ created "), "{created}");
    assert_eq!(lines[1], "  local agents: spec (agent.toml)", "{created}");
}

#[test]
fn the_creation_line_lists_local_instructions_and_schemas() {
    let sandbox = project(&[spec(1, "# Spec 1\n")]);
    let feedback = "schemas/feedback.schema.json";
    let built_in = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/agents/reviewer")
        .join(feedback);
    let schema = fs::read_to_string(built_in).expect("the built-in feedback schema");
    write_local_file(
        &sandbox,
        "reviewer",
        "instructions.md",
        "Follow our style guide.\n",
    );
    write_local_file(&sandbox, "reviewer", feedback, &schema);

    let created = ok(sandbox.feature(&["add login"]));

    let lines: Vec<&str> = created.lines().take(2).collect();
    assert_eq!(
        lines[1], "  local agents: reviewer (instructions.md, schemas/feedback.schema.json)",
        "{created}"
    );
}

#[test]
fn a_new_run_lists_the_agents_that_use_local_files_under_its_run_line() {
    let sandbox = project(&[spec(1, "# Spec 1\n")]);
    let name = feature_name();
    ok(sandbox.feature(&["add login"]));
    ok(surt(&sandbox, &["cancel", "--yes", &name]));
    write_local(
        &sandbox,
        "spec",
        &format!("[profile]\nmodel = \"{LOCAL_MODEL}\"\n"),
    );

    let started = ok(surt(&sandbox, &["feature", &name, "--new-run"]));

    let lines: Vec<&str> = started.lines().take(2).collect();
    assert!(lines[0].starts_with("✔ run 2 of "), "{started}");
    assert_eq!(lines[1], "  local agents: spec (agent.toml)", "{started}");
}

#[test]
fn without_local_files_the_creation_line_stands_alone() {
    let sandbox = project(&[spec(1, "# Spec 1\n")]);

    let created = ok(sandbox.feature(&["add login"]));

    assert!(!created.contains("local agents"), "{created}");
}

#[test]
fn an_invalid_override_refuses_the_feature_with_exit_3_before_any_row_is_written() {
    let sandbox = project(&[spec(1, "# Spec 1\n")]);
    write_local(&sandbox, "builder", "[profile]\neffort = \"huge\"\n");

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
    assert_eq!(
        sandbox.texts("SELECT CAST(count(*) AS TEXT) FROM subject"),
        ["0"]
    );
}

#[test]
fn plan_and_start_drive_a_feature_whose_builder_has_a_local_model() {
    let sandbox = project(&[
        spec(1, "# Spec 1\n"),
        tasks(1, &["A"]),
        build("A", 1, 1, "login.txt", "login\n"),
        review("A", 1, "approved"),
        ship(),
    ]);
    write_local(
        &sandbox,
        "builder",
        &format!("[profile]\nmodel = \"{LOCAL_MODEL}\"\n"),
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
        [LOCAL_MODEL]
    );
}
