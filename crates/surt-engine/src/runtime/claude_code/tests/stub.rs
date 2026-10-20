use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::json;

use super::executions::{CONTEXT, Execution, INSTRUCTIONS, NOTE};
use super::process::{parent_environment, run};
use super::streams::{Fixture, Stream, delivered_note};
use crate::project::GlobalConfig;
use crate::runtime::{ClaudeCodeRuntime, Exit, FailureClass, RuntimeEvent, StopSignal, Verdict};

const SHORT_GRACE: Duration = Duration::from_millis(200);
const PROMPT_STOP: Duration = Duration::from_secs(5);
const REAP_WAIT: Duration = Duration::from_millis(50);
const REAP_ATTEMPTS: u32 = 100;

const SUPPORTED_VERSION: &str = "2.1.301";

struct Stub {
    executable: PathBuf,
    record: PathBuf,
}

impl Stub {
    fn new(execution: &Execution, stream: &Stream, tail: &str) -> Stub {
        Stub::around(execution, stream, "", tail)
    }

    fn with_grandchild(execution: &Execution, stream: &Stream, signals: &str) -> Stub {
        let record = execution.root().join("record");
        let head = format!(
            "{signals}\nsleep 30 &\necho $! > '{}/grandchild'",
            record.display()
        );
        Stub::around(execution, stream, &head, "wait")
    }

    fn around(execution: &Execution, stream: &Stream, head: &str, tail: &str) -> Stub {
        Stub::answering(execution, stream, SUPPORTED_VERSION, head, tail)
    }

    fn answering(
        execution: &Execution,
        stream: &Stream,
        version: &str,
        head: &str,
        tail: &str,
    ) -> Stub {
        let root = execution.root();
        let record = root.join("record");
        fs::create_dir(&record).expect("the record directory");
        let stream_file = root.join("stream.jsonl");
        fs::write(&stream_file, stream.lines().join("\n") + "\n").expect("the replayed stream");
        let record_text = record.display();
        let script = format!(
            "#!/bin/sh\n\
             if [ \"$1\" = --version ]; then echo '{version} (Claude Code)'; exit 0; fi\n\
             {head}\n\
             printf '%s\\n' \"$@\" > '{record_text}/argv'\n\
             env > '{record_text}/env'\n\
             pwd > '{record_text}/cwd'\n\
             cat > '{record_text}/stdin'\n\
             cat '{}'\n\
             {tail}\n",
            stream_file.display()
        );
        let executable = root.join("claude");
        fs::write(&executable, script).expect("the stub executable");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("an executable stub");
        Stub { executable, record }
    }

    fn recorded(&self, name: &str) -> String {
        fs::read_to_string(self.record.join(name)).expect("a recorded file")
    }

    fn runtime(&self, extra: &[(&str, &str)]) -> ClaudeCodeRuntime {
        self.configured(&GlobalConfig::default(), extra)
    }

    fn configured(&self, global: &GlobalConfig, extra: &[(&str, &str)]) -> ClaudeCodeRuntime {
        ClaudeCodeRuntime::new(
            &self.executable,
            parent_environment(extra),
            global.stop_grace_period,
        )
    }
}

fn write_note(execution: &Execution) -> String {
    format!(
        "printf '{}' > '{}/note.md'",
        NOTE.replace('\n', "\\n"),
        execution.request.delivery_dir.display()
    )
}

fn is_alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid.trim()])
        .status()
        .expect("kill runs")
        .success()
}

fn assert_ends(pid: &str) {
    let ended = (0..REAP_ATTEMPTS).any(|_| {
        std::thread::sleep(REAP_WAIT);
        !is_alive(pid)
    });
    assert!(ended, "process {pid} outlived the stop");
}

#[tokio::test]
async fn a_replayed_success_is_followed_until_the_process_ends_and_accepted() {
    let stream = delivered_note().last(json!({ "type": "system", "subtype": "hook_response" }));
    let execution = Execution::new(stream.session_id());
    let stub = Stub::new(&execution, &stream, &write_note(&execution));

    let run = run(&stub.runtime(&[]), &execution, |_| {}).await;

    let completion = run.outcome.expect("a completion");
    assert_eq!(completion.exit, Exit::Code(0));
    assert!(matches!(
        execution.verdict(&completion),
        Verdict::Accepted(_)
    ));
    assert!(matches!(
        run.events.as_slice(),
        [
            RuntimeEvent::Identified { version },
            RuntimeEvent::Launched { pid: Some(_) },
            RuntimeEvent::Initialized(_),
            RuntimeEvent::Reported(report)
        ] if version == SUPPORTED_VERSION
            && report.exit_code == Some(0)
            && report.turns == Some(3)
    ));
}

#[tokio::test]
async fn the_stream_and_stderr_are_appended_to_the_execution_logs() {
    let stream = delivered_note();
    let execution = Execution::new(stream.session_id());
    let logs = execution.root().join("logs");
    fs::create_dir(&logs).expect("the log directory");
    fs::write(logs.join("stream.jsonl"), "earlier\n").expect("an earlier stream");
    let stub = Stub::new(&execution, &stream, "echo 'a diagnostic' >&2");

    run(&stub.runtime(&[]), &execution, |_| {}).await;

    let logged = fs::read_to_string(logs.join("stream.jsonl")).expect("the stream log");
    assert_eq!(logged, format!("earlier\n{}\n", stream.lines().join("\n")));
    let stderr = fs::read_to_string(logs.join("stderr.log")).expect("the stderr log");
    assert_eq!(stderr, "a diagnostic\n");
}

#[tokio::test]
async fn the_context_goes_through_stdin_with_the_assembled_launch() {
    let stream = delivered_note();
    let execution = Execution::new(stream.session_id());
    let stub = Stub::new(&execution, &stream, "");

    run(&stub.runtime(&[]), &execution, |_| {}).await;

    assert_eq!(stub.recorded("stdin"), CONTEXT);
    assert_eq!(
        Path::new(stub.recorded("cwd").trim()),
        execution.root().join("worktree")
    );
    let argv = stub.recorded("argv");
    let expected: String = execution
        .invocation(&[])
        .arguments
        .iter()
        .map(|argument| format!("{}\n", argument.to_string_lossy()))
        .collect();
    assert_eq!(argv, expected);
    let instructions = execution.root().join("launch").join("instructions.md");
    assert_eq!(
        fs::read_to_string(instructions).expect("instructions"),
        INSTRUCTIONS
    );
}

#[tokio::test]
async fn the_child_environment_drops_everything_outside_the_allowlist() {
    let stream = delivered_note();
    let execution = Execution::new(stream.session_id());
    let stub = Stub::new(&execution, &stream, "");
    let inherited = [
        ("ANTHROPIC_BASE_URL", "https://gateway.example"),
        ("ANTHROPIC_API_KEY", "sk-live"),
        ("CLAUDE_CODE_SIMPLE", "1"),
        ("LANG", "en_US.UTF-8"),
    ];

    run(&stub.runtime(&inherited), &execution, |_| {}).await;

    let environment = stub.recorded("env");
    let names: Vec<&str> = environment
        .lines()
        .filter_map(|line| line.split_once('=').map(|(name, _)| name))
        .collect();
    for leaked in [
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_API_KEY",
        "CLAUDE_CODE_SIMPLE",
    ] {
        assert!(!names.contains(&leaked), "{leaked} leaked into {names:?}");
    }
    for line in [
        "LANG=en_US.UTF-8",
        "DISABLE_AUTOUPDATER=1",
        "CLAUDE_CODE_DISABLE_CLAUDE_MDS=1",
    ] {
        assert!(
            environment.lines().any(|seen| seen == line),
            "{line} missing"
        );
    }
}

#[tokio::test]
async fn a_failure_reported_on_the_stream_is_classified_after_exit() {
    let stream = Stream::captured(Fixture::AuthExpired);
    let execution = Execution::new(stream.session_id());
    let stub = Stub::new(&execution, &stream, "exit 1");

    let run = run(&stub.runtime(&[]), &execution, |_| {}).await;

    let error = run.outcome.expect_err("an auth failure");
    assert_eq!(error.class, FailureClass::NotRetryable);
    assert_eq!(error.code, "authentication_failed");
    assert!(matches!(
        run.events.last(),
        Some(RuntimeEvent::Reported(report))
            if report.exit_code == Some(1) && report.turns == Some(1)
    ));
}

#[tokio::test]
async fn a_process_killed_by_a_signal_reports_the_shell_convention() {
    let stream = Stream::captured(Fixture::Sigterm);
    let execution = Execution::new(stream.session_id());
    let stub = Stub::new(&execution, &stream, "kill -TERM $$");

    let run = run(&stub.runtime(&[]), &execution, |_| {}).await;

    let completion = run.outcome.expect("a completion");
    assert_eq!(completion.exit, Fixture::Sigterm.exit());
    assert!(completion.result.is_none());
    assert!(matches!(
        run.events.last(),
        Some(RuntimeEvent::Reported(report))
            if report.exit_code == Some(143) && report.turns.is_none()
    ));
}

#[tokio::test]
async fn a_graceful_stop_ends_the_whole_process_tree_and_interrupts() {
    let stream = Stream::captured(Fixture::Sigterm);
    let execution = Execution::new(stream.session_id());
    let stub = Stub::with_grandchild(&execution, &stream, "");

    let started = Instant::now();
    let run = run(&stub.runtime(&[]), &execution, StopSignal::graceful).await;

    let completion = run.outcome.expect("a completion");
    assert_eq!(completion.exit, Exit::Stopped);
    assert!(matches!(
        execution.verdict(&completion),
        Verdict::Interrupted
    ));
    assert!(started.elapsed() < PROMPT_STOP, "{:?}", started.elapsed());
    assert_ends(&stub.recorded("grandchild"));
}

#[tokio::test]
async fn a_tree_ignoring_the_graceful_stop_is_killed_after_the_configured_grace_period() {
    let stream = Stream::captured(Fixture::Sigterm);
    let execution = Execution::new(stream.session_id());
    let stub = Stub::with_grandchild(&execution, &stream, "trap '' TERM");
    let global = GlobalConfig {
        stop_grace_period: SHORT_GRACE,
    };
    let runtime = stub.configured(&global, &[]);

    let started = Instant::now();
    let run = run(&runtime, &execution, StopSignal::graceful).await;

    let completion = run.outcome.expect("a completion");
    assert_eq!(completion.exit, Exit::Stopped);
    assert!(started.elapsed() >= SHORT_GRACE, "{:?}", started.elapsed());
    assert!(
        started.elapsed() < PROMPT_STOP,
        "killed after {:?}, not after the configured {SHORT_GRACE:?}",
        started.elapsed()
    );
    assert_ends(&stub.recorded("grandchild"));
}

#[tokio::test]
async fn a_forced_stop_kills_the_tree_without_waiting_for_the_grace_period() {
    let stream = Stream::captured(Fixture::Sigterm);
    let execution = Execution::new(stream.session_id());
    let stub = Stub::with_grandchild(&execution, &stream, "trap '' TERM");

    let started = Instant::now();
    let run = run(&stub.runtime(&[]), &execution, StopSignal::force).await;

    assert_eq!(run.outcome.expect("a completion").exit, Exit::Stopped);
    assert!(started.elapsed() < PROMPT_STOP, "{:?}", started.elapsed());
    assert_ends(&stub.recorded("grandchild"));
}

#[tokio::test]
async fn an_incompatible_initialization_stops_the_process_and_fails() {
    let stream = delivered_note().with_init("model", json!("claude-opus-5-5"));
    let execution = Execution::new(stream.session_id());
    let stub = Stub::new(&execution, &stream, "sleep 30");

    let started = Instant::now();
    let run = run(&stub.runtime(&[]), &execution, |_| {}).await;

    let error = run.outcome.expect_err("an incompatible initialization");
    assert_eq!(error.code, "model_mismatch");
    assert_eq!(error.class, FailureClass::NotRetryable);
    assert!(started.elapsed() < PROMPT_STOP, "{:?}", started.elapsed());
}

#[tokio::test]
async fn an_outdated_claude_code_is_refused_before_launch_and_its_version_reported() {
    let stream = delivered_note();
    let execution = Execution::new(stream.session_id());
    let stub = Stub::answering(&execution, &stream, "2.1.300", "", "");

    let run = run(&stub.runtime(&[]), &execution, |_| {}).await;

    let error = run.outcome.expect_err("an outdated Claude Code");
    assert_eq!(error.class, FailureClass::NotRetryable);
    assert_eq!(error.code, "unsupported_version");
    assert_eq!(
        error.message,
        "Claude Code 2.1.300 is older than the supported 2.1.301; upgrade it with `claude update`"
    );
    assert_eq!(
        run.events,
        [RuntimeEvent::Identified {
            version: "2.1.300".to_owned()
        }]
    );
    assert!(!stub.record.join("argv").exists());
}

#[tokio::test]
async fn a_version_answer_without_a_version_is_refused_with_an_install_message() {
    let stream = delivered_note();
    let execution = Execution::new(stream.session_id());
    let stub = Stub::answering(&execution, &stream, "unknown", "", "");

    let run = run(&stub.runtime(&[]), &execution, |_| {}).await;

    let error = run.outcome.expect_err("an unreadable version");
    assert_eq!(error.class, FailureClass::NotRetryable);
    assert_eq!(error.code, "unsupported_version");
    assert!(
        error
            .message
            .ends_with("install Claude Code 2.1.301 or newer"),
        "{}",
        error.message
    );
    assert!(run.events.is_empty());
}
