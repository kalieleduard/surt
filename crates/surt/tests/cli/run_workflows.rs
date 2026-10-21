use std::fs;

use crate::run::{project, subjects, today};
use crate::scripts::spec;
use crate::support::Sandbox;

const HELLO_WORKFLOW: &str = r#"schema = 1
name = "hello"

[steps.greet]
kind = "agent"
agent = "greeter"
"#;

const GREETER_AGENT: &str = r#"schema = 1
id = "greeter"

[outputs.note]
format = "text"

[profile]
runtime = "claude-code"
model = "claude-sonnet-5-5"
effort = "high"

[profile.claude]
auth = "subscription"
permission_mode = "dontAsk"
tools = ["Read"]
allowed_tools = ["Read"]
disallowed_tools = []
"#;

const GREETING: &str = "[[on]]\nstep = \"greet\"\n\
     deliver = { outcome = \"done\", outputs = { note = \"note.txt\" }, files = { \"note.txt\" = \"hello\" } }\n\n";

fn with_hello(sandbox: &Sandbox) {
    let dot_surt = sandbox.dot_surt();
    let agent = dot_surt.join("agents").join("greeter");
    fs::create_dir_all(&agent).expect("the agent directory");
    fs::write(agent.join("agent.toml"), GREETER_AGENT).expect("the agent");
    fs::write(agent.join("instructions.md"), "Say hello.\n").expect("the instructions");
    fs::write(
        dot_surt.join("workflows").join("hello.toml"),
        HELLO_WORKFLOW,
    )
    .expect("the workflow");
    sandbox.script(&[GREETING.to_owned()]);
}

#[test]
fn a_workflow_without_inputs_runs_and_is_numbered() {
    let sandbox = project();
    with_hello(&sandbox);

    let run = sandbox.surt(&sandbox.repo(), &["run", "hello"]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let name = format!("{}-1", today());
    assert!(
        run.stderr
            .starts_with(&format!("✔ created {name}   workflow: hello\n")),
        "{}",
        run.stderr
    );
    assert_eq!(subjects(&sandbox), [name]);
    assert_eq!(
        sandbox.texts("SELECT state FROM workflow_run"),
        ["succeeded"]
    );
}

#[test]
fn numbering_is_shared_between_workflows() {
    let sandbox = project();
    with_hello(&sandbox);
    sandbox.script(&[
        spec(
            1, "# Spec
",
        ),
        GREETING.to_owned(),
    ]);
    sandbox.run_feature_unnamed("add login", &[]);
    sandbox.run_feature_unnamed("add logout", &[]);

    let run = sandbox.surt(&sandbox.repo(), &["run", "hello"]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let today = today();
    assert!(
        run.stderr.starts_with(&format!(
            "✔ created {today}-3   workflow: hello
"
        )),
        "{}",
        run.stderr
    );
    assert_eq!(
        subjects(&sandbox),
        [
            format!("{today}-1"),
            format!("{today}-2"),
            format!("{today}-3")
        ]
    );
}

#[test]
fn an_input_given_to_a_workflow_without_inputs_is_refused() {
    let sandbox = project();
    with_hello(&sandbox);

    let run = sandbox.surt(
        &sandbox.repo(),
        &["run", "hello", "--input", "request=add login"],
    );

    assert_eq!(
        run.stderr,
        "✖ workflow hello has no input \"request\"\n  workflow hello takes no inputs\n  \
         fix: drop --input request, or declare it in .surt/workflows/hello.toml\n"
    );
    assert!(subjects(&sandbox).is_empty());
}
