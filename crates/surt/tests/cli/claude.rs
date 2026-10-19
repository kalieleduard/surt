use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::support::Sandbox;

const BLOCKED_EXIT_CODE: i32 = 4;
const UNKNOWN_EVENT: &str = r#"{"type":"system","subtype":"ui_invalidate"}"#;

fn project() -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox
}

fn install_claude(sandbox: &Sandbox, version: &str, run: &str) {
    let tools = sandbox.tools();
    fs::create_dir_all(&tools).expect("the tools directory");
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = --version ]; then echo '{version} (Claude Code)'; exit 0; fi\n\
         {run}\n"
    );
    let claude = tools.join("claude");
    fs::write(&claude, script).expect("the stub Claude Code");
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).expect("an executable stub");
}

fn executions(sandbox: &Sandbox) -> Vec<String> {
    sandbox.texts(
        "SELECT state || ' ' || failure_class || ' ' || error_code FROM execution ORDER BY attempt",
    )
}

#[test]
fn without_a_scripted_runtime_a_missing_claude_code_blocks_the_step_at_once() {
    let sandbox = project();

    let run = sandbox.surt_on_claude(&["run", "feature", "--input", "request=add login"]);

    assert_eq!(run.code, Some(BLOCKED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        executions(&sandbox),
        ["failed not_retryable missing_executable"]
    );
    assert!(
        run.stderr.contains("install Claude Code 2.1.301 or newer"),
        "{}",
        run.stderr
    );
}

#[test]
fn an_outdated_claude_code_blocks_the_step_at_once_and_its_version_is_recorded() {
    let sandbox = project();
    install_claude(&sandbox, "2.0.14", "exit 0");

    let run = sandbox.surt_on_claude(&["run", "feature", "--input", "request=add login"]);

    assert_eq!(run.code, Some(BLOCKED_EXIT_CODE), "{}", run.stderr);
    assert_eq!(
        executions(&sandbox),
        ["failed not_retryable unsupported_version"]
    );
    assert!(
        run.stderr.contains(
            "Claude Code 2.0.14 is older than the supported 2.1.301; upgrade it with `claude update`"
        ),
        "{}",
        run.stderr
    );
    assert_eq!(
        sandbox.texts("SELECT json_extract(runtime_info, '$.runtime_version') FROM execution"),
        ["2.0.14"]
    );
}

#[test]
fn each_execution_writes_its_stream_and_stderr_to_the_feature_log_directory() {
    let sandbox = project();
    install_claude(
        &sandbox,
        "2.1.301",
        &format!("echo '{UNKNOWN_EVENT}'\necho 'a diagnostic' >&2\nexit 1"),
    );

    let run = sandbox.surt_on_claude(&["run", "feature", "--input", "request=add login"]);

    assert_eq!(run.code, Some(BLOCKED_EXIT_CODE), "{}", run.stderr);
    let subject = sandbox.texts("SELECT name FROM subject").concat();
    let ids = sandbox.texts("SELECT id FROM execution ORDER BY attempt");
    assert_eq!(ids.len(), 3, "{}", run.stderr);
    for id in ids {
        let logs = sandbox.dot_surt().join("logs").join(&subject).join(&id);
        assert_eq!(
            fs::read_to_string(logs.join("stream.jsonl")).expect("the stream log"),
            format!("{UNKNOWN_EVENT}\n")
        );
        assert_eq!(
            fs::read_to_string(logs.join("stderr.log")).expect("the stderr log"),
            "a diagnostic\n"
        );
    }
    assert_eq!(
        sandbox.texts(
            "SELECT DISTINCT json_extract(runtime_info, '$.runtime_version') FROM execution"
        ),
        ["2.1.301"]
    );
}
