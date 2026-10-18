use std::ffi::OsString;

use tokio::sync::mpsc;
use uuid::Uuid;

use super::executions::Execution;
use crate::runtime::{
    AgentRuntime, ClaudeCodeRuntime, FailureClass, RuntimeCompletion, RuntimeError, RuntimeEvent,
    StopSignal,
};

pub(super) fn parent_environment(extra: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
    let inherited = ["PATH", "HOME"]
        .into_iter()
        .filter_map(|name| Some((OsString::from(name), std::env::var_os(name)?)));
    let extra = extra
        .iter()
        .map(|(name, value)| (OsString::from(name), OsString::from(value)));
    inherited.chain(extra).collect()
}

pub(super) struct Run {
    pub(super) outcome: Result<RuntimeCompletion, RuntimeError>,
    pub(super) events: Vec<RuntimeEvent>,
}

pub(super) async fn run(
    runtime: &ClaudeCodeRuntime,
    execution: &Execution,
    on_initialized: impl Fn(&StopSignal),
) -> Run {
    let stop = StopSignal::new();
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let watcher = async {
        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            if matches!(event, RuntimeEvent::Initialized(_)) {
                on_initialized(&stop);
            }
            events.push(event);
        }
        events
    };
    let execute = runtime.execute(execution.request.clone(), sender, stop.clone());
    let (outcome, events) = tokio::join!(execute, watcher);
    Run { outcome, events }
}

#[tokio::test]
async fn a_missing_executable_is_not_retryable() {
    let execution = Execution::new(Uuid::now_v7());
    let runtime = ClaudeCodeRuntime::new(
        execution.root().join("no-such-claude"),
        parent_environment(&[]),
    );

    let run = run(&runtime, &execution, |_| {}).await;

    let error = run.outcome.expect_err("a launch failure");
    assert_eq!(error.class, FailureClass::NotRetryable);
    assert_eq!(error.code, "missing_executable");
    assert!(run.events.is_empty());
}

#[tokio::test]
async fn a_missing_working_directory_fails_before_launch() {
    let execution = Execution::new(Uuid::now_v7());
    std::fs::remove_dir(execution.root().join("worktree")).expect("the working directory");
    let runtime = ClaudeCodeRuntime::new(
        execution.root().join("no-such-claude"),
        parent_environment(&[]),
    );

    let run = run(&runtime, &execution, |_| {}).await;

    let error = run.outcome.expect_err("a launch failure");
    assert_eq!(error.class, FailureClass::Retryable);
    assert_eq!(error.code, "missing_working_dir");
}
