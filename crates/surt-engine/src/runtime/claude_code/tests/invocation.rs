use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

use serde_json::{Value, json};
use uuid::Uuid;

use super::executions::{CONTEXT, Execution, INSTRUCTIONS};
use crate::runtime::FailureClass;
use crate::runtime::claude_code::rules::absolute_pattern;

const BUILDER_AGENT: &str = r#"
schema = 1
id = "noter"

[outputs.note]
format = "markdown"

[profile]
runtime = "claude-code"
model = "claude-sonnet-5-5"
effort = "high"

[profile.limits]
max_turns = 100
max_budget_usd = 5.0

[profile.claude]
auth = "subscription"
permission_mode = "bypassPermissions"
tools = ["Read", "Glob", "Grep", "Write", "Edit", "Bash"]
allowed_tools = ["Read", "Glob", "Grep", "Bash(git status *)", "Bash(cargo test *)"]
disallowed_tools = ["mcp__*", "Bash(git push *)"]
"#;

const API_KEY_AGENT: &str = r#"
schema = 1
id = "noter"

[outputs.note]
format = "markdown"

[profile]
runtime = "claude-code"
model = "claude-haiku-5-5"
effort = "max"

[profile.claude]
auth = "api_key"
api_key_env = "SURT_TEST_KEY"
permission_mode = "dontAsk"
tools = []
allowed_tools = []
disallowed_tools = []
"#;

const FAKE_AGENT: &str = r#"
schema = 1
id = "noter"

[outputs.note]
format = "markdown"

[profile]
runtime = "fake"
model = "claude-haiku-5-5"
effort = "low"
"#;

fn strings(arguments: &[OsString]) -> Vec<String> {
    arguments
        .iter()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

fn path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn edit_rule(root: &Path) -> String {
    format!("Edit({}/**)", absolute_pattern(&path(root)))
}

fn value_after<'a>(arguments: &'a [String], flag: &str) -> &'a str {
    let at = arguments
        .iter()
        .position(|argument| argument == flag)
        .unwrap_or_else(|| panic!("{flag} is passed"));
    &arguments[at + 1]
}

fn environment(execution: &Execution, parent: &[(&str, &str)]) -> BTreeMap<String, String> {
    execution
        .invocation(parent)
        .environment
        .into_iter()
        .map(|(name, value)| {
            (
                name.to_string_lossy().into_owned(),
                value.to_string_lossy().into_owned(),
            )
        })
        .collect()
}

#[test]
fn absolute_roots_are_rendered_from_the_filesystem_root() {
    assert_eq!(
        absolute_pattern("/tmp/surt-fixture/delivery"),
        "//tmp/surt-fixture/delivery"
    );
    assert_eq!(
        absolute_pattern(r"C:\Users\dev\project\delivery"),
        "//c/Users/dev/project/delivery"
    );
    assert_eq!(
        absolute_pattern(r"\\?\D:\work\delivery"),
        "//d/work/delivery"
    );
}

#[test]
fn a_subscription_launch_isolates_a_fresh_session() {
    let session_id = Uuid::now_v7();
    let execution = Execution::new(session_id);
    let launch_dir = execution.root().join("launch");

    let arguments = strings(&execution.invocation(&[]).arguments);

    let expected: Vec<String> = [
        "-p",
        "--session-id",
        &session_id.to_string(),
        "--no-session-persistence",
        "--output-format",
        "stream-json",
        "--verbose",
        "--model",
        "claude-haiku-5-5",
        "--effort",
        "low",
        "--permission-mode",
        "dontAsk",
        "--permission-prompts",
        "none",
        "--tools",
        "Read,Write",
        "--allowedTools",
        "Read",
        &edit_rule(&execution.request.delivery_dir),
        "--disallowedTools",
        "Bash",
        "mcp__*",
        "--json-schema",
        value_after(&arguments, "--json-schema"),
        "--append-system-prompt-file",
        &path(&launch_dir.join("instructions.md")),
        "--settings",
        &path(&launch_dir.join("settings.json")),
        "--strict-mcp-config",
        "--mcp-config",
        &path(&launch_dir.join("mcp.json")),
        "--disable-slash-commands",
        "--safe-mode",
        "--setting-sources",
        "user",
    ]
    .map(str::to_owned)
    .to_vec();
    assert_eq!(arguments, expected);
}

#[test]
fn the_json_schema_is_generated_from_the_frozen_contract() {
    let execution = Execution::new(Uuid::now_v7());
    let arguments = strings(&execution.invocation(&[]).arguments);

    let schema: Value =
        serde_json::from_str(value_after(&arguments, "--json-schema")).expect("a JSON schema");

    assert_eq!(
        schema,
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["outcome", "outputs"],
            "properties": {
                "outcome": { "enum": ["done"] },
                "outputs": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["note"],
                    "properties": { "note": { "type": "string" } }
                }
            }
        })
    );
}

#[test]
fn the_launch_files_disable_hooks_and_mcp_and_carry_the_instructions() {
    let execution = Execution::new(Uuid::now_v7());
    let launch_dir = execution.root().join("launch");

    let invocation = execution.invocation(&[]);

    let files: BTreeMap<_, _> = invocation.files.into_iter().collect();
    let json_file = |name: &str| -> Value {
        serde_json::from_str(&files[&launch_dir.join(name)]).expect("a JSON launch file")
    };
    assert_eq!(files.len(), 3);
    assert_eq!(files[&launch_dir.join("instructions.md")], INSTRUCTIONS);
    assert_eq!(
        json_file("settings.json"),
        json!({ "disableAllHooks": true, "switchModelsOnFlag": false })
    );
    assert_eq!(json_file("mcp.json"), json!({ "mcpServers": {} }));
    assert_eq!(invocation.stdin, CONTEXT);
    assert_eq!(invocation.working_dir, execution.root().join("worktree"));
}

#[test]
fn an_editing_profile_may_also_edit_its_working_directory_within_its_limits() {
    let execution = Execution::with_agent(BUILDER_AGENT, Uuid::now_v7());

    let arguments = strings(&execution.invocation(&[]).arguments);

    let between = |from: &str, to: &str| -> Vec<&str> {
        let start = arguments.iter().position(|a| a == from).expect("from") + 1;
        let end = arguments.iter().position(|a| a == to).expect("to");
        arguments[start..end].iter().map(String::as_str).collect()
    };
    assert_eq!(
        value_after(&arguments, "--tools"),
        "Read,Glob,Grep,Write,Edit,Bash"
    );
    assert_eq!(
        value_after(&arguments, "--permission-mode"),
        "bypassPermissions"
    );
    assert_eq!(value_after(&arguments, "--effort"), "high");
    assert_eq!(
        between("--allowedTools", "--disallowedTools"),
        [
            "Read",
            "Glob",
            "Grep",
            "Bash(git status *)",
            "Bash(cargo test *)",
            &edit_rule(&execution.request.delivery_dir),
            &edit_rule(&execution.root().join("worktree")),
        ]
    );
    assert_eq!(
        between("--disallowedTools", "--json-schema"),
        ["mcp__*", "Bash(git push *)"]
    );
    assert_eq!(value_after(&arguments, "--max-turns"), "100");
    assert_eq!(value_after(&arguments, "--max-budget-usd"), "5");
}

#[test]
fn an_api_key_launch_is_bare_and_takes_the_key_from_the_named_variable() {
    let execution = Execution::with_agent(API_KEY_AGENT, Uuid::now_v7());

    let invocation = execution.invocation(&[("SURT_TEST_KEY", "sk-test"), ("HOME", "/home/dev")]);

    let arguments = strings(&invocation.arguments);
    assert_eq!(arguments.last().map(String::as_str), Some("--bare"));
    assert!(!arguments.iter().any(|argument| argument == "--safe-mode"));
    assert_eq!(value_after(&arguments, "--tools"), "");
    assert_eq!(value_after(&arguments, "--effort"), "max");
    assert_eq!(
        environment(&execution, &[("SURT_TEST_KEY", "sk-test")]).get("ANTHROPIC_API_KEY"),
        Some(&"sk-test".to_owned())
    );
}

#[test]
fn an_api_key_launch_without_its_variable_is_not_retryable() {
    let execution = Execution::with_agent(API_KEY_AGENT, Uuid::now_v7());
    let error = crate::runtime::claude_code::Invocation::assemble(&execution.request, &[])
        .expect_err("a missing key");

    assert_eq!(error.class, FailureClass::NotRetryable);
    assert_eq!(error.code, "missing_api_key");
}

#[test]
fn a_profile_without_a_claude_table_cannot_launch() {
    let execution = Execution::with_agent(FAKE_AGENT, Uuid::now_v7());
    let error = crate::runtime::claude_code::Invocation::assemble(&execution.request, &[])
        .expect_err("an unsupported profile");

    assert_eq!(error.class, FailureClass::NotRetryable);
    assert_eq!(error.code, "unsupported_profile");
}

#[test]
fn the_environment_is_an_allowlist_plus_the_isolation_switches() {
    let execution = Execution::new(Uuid::now_v7());
    let parent = [
        ("HOME", "/home/dev"),
        ("PATH", "/usr/bin:/bin"),
        ("USER", "dev"),
        ("LANG", "en_US.UTF-8"),
        ("TMPDIR", "/tmp/dev"),
        ("SystemRoot", r"C:\Windows"),
        ("ANTHROPIC_API_KEY", "sk-live"),
        ("ANTHROPIC_AUTH_TOKEN", "token"),
        ("ANTHROPIC_BASE_URL", "https://gateway.example"),
        ("ANTHROPIC_MODEL", "claude-opus-5-5"),
        ("CLAUDE_CODE_EFFORT_LEVEL", "max"),
        ("CLAUDE_CODE_SIMPLE", "1"),
        ("CLAUDE_CODE_USE_BEDROCK", "1"),
        ("CLAUDE_CODE_OAUTH_TOKEN", "oauth"),
        ("CLAUDECODE", "1"),
        ("DISABLE_AUTOUPDATER", "0"),
        ("GIT_DIR", "/elsewhere/.git"),
    ];

    let child = environment(&execution, &parent);

    let expected: BTreeMap<String, String> = [
        ("HOME", "/home/dev"),
        ("PATH", "/usr/bin:/bin"),
        ("USER", "dev"),
        ("LANG", "en_US.UTF-8"),
        ("TMPDIR", "/tmp/dev"),
        ("SystemRoot", r"C:\Windows"),
        ("DISABLE_AUTOUPDATER", "1"),
        ("DISABLE_UPDATES", "1"),
        ("CLAUDE_CODE_DISABLE_AUTO_MEMORY", "1"),
        ("CLAUDE_CODE_DISABLE_CLAUDE_MDS", "1"),
        ("CLAUDE_CODE_STARTUP_FAILURE_RESULTS", "1"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect();
    assert_eq!(child, expected);
}

#[test]
fn never_resumes_or_continues_a_session() {
    let execution = Execution::with_agent(BUILDER_AGENT, Uuid::now_v7());

    let arguments = strings(&execution.invocation(&[]).arguments);

    for forbidden in [
        "--resume",
        "--continue",
        "--fork-session",
        "--fallback-model",
    ] {
        assert!(
            !arguments.iter().any(|argument| argument == forbidden),
            "{forbidden}"
        );
    }
    assert_eq!(
        arguments
            .iter()
            .filter(|argument| *argument == "--session-id")
            .count(),
        1
    );
}

#[test]
fn every_execution_writes_its_rules_with_its_own_roots() {
    let execution = Execution::new(Uuid::now_v7());

    let arguments = strings(&execution.invocation(&[]).arguments);

    assert!(
        arguments
            .iter()
            .all(|argument| !argument.starts_with("Write("))
    );
    let rule = edit_rule(&execution.request.delivery_dir);
    assert!(rule.starts_with("Edit(//"), "{rule}");
    assert!(arguments.contains(&rule));
}
