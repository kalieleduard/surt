use uuid::Uuid;

use super::executions::{CONTEXT, DELTA, Execution, PREVIOUS_SESSION};
use super::invocation::strings;
use crate::runtime::claude_code::{Invocation, restart};

const NO_PERSISTENCE: &str = "--no-session-persistence";

fn launched(execution: &Execution) -> (Vec<String>, String) {
    let invocation = execution.invocation(&[]);
    (strings(&invocation.arguments), invocation.stdin)
}

#[test]
fn a_loop_agent_with_nothing_to_continue_keeps_its_session_and_reads_the_full_context() {
    let session_id = Uuid::now_v7();
    let execution = Execution::new(session_id);
    let (without_conversation, _) = launched(&execution);

    let (arguments, stdin) = launched(&execution.in_a_loop());

    let expected: Vec<String> = without_conversation
        .into_iter()
        .filter(|argument| argument != NO_PERSISTENCE)
        .collect();
    assert_eq!(arguments, expected);
    assert_eq!(
        arguments[..3],
        ["-p", "--session-id", &session_id.to_string()]
    );
    assert_eq!(stdin, CONTEXT);
}

#[test]
fn a_continued_launch_forks_the_previous_session_and_reads_only_the_delta() {
    let session_id = Uuid::now_v7();
    let execution = Execution::new(session_id).in_a_loop();
    let (fresh, _) = launched(&execution);

    let (arguments, stdin) = launched(&execution.continuing());

    let mut expected = fresh;
    expected.splice(
        3..3,
        ["--resume", PREVIOUS_SESSION, "--fork-session"].map(str::to_owned),
    );
    assert_eq!(arguments, expected);
    assert_eq!(
        arguments[..6],
        [
            "-p",
            "--session-id",
            &session_id.to_string(),
            "--resume",
            PREVIOUS_SESSION,
            "--fork-session"
        ]
    );
    assert!(!arguments.iter().any(|argument| argument == NO_PERSISTENCE));
    assert_eq!(stdin, DELTA);
}

#[test]
fn a_restarted_launch_is_the_fresh_loop_launch_with_the_same_session() {
    let execution = Execution::new(Uuid::now_v7()).in_a_loop();
    let (fresh, _) = launched(&execution);
    let execution = execution.continuing();

    let restarted = Invocation::assemble(&restart::fresh(&execution.request), &[])
        .expect("a restarted invocation");

    assert_eq!(strings(&restarted.arguments), fresh);
    assert_eq!(restarted.stdin, CONTEXT);
    assert_eq!(
        restarted.expectation.session_id,
        execution.request.session_id
    );
}
