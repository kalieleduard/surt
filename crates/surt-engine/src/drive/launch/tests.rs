use super::Ending;
use crate::drive::failure::Failure;
use crate::drive::pausing::AgentEnd;
use crate::runtime::FailureClass;
use crate::store::ExecutionEnd;

const LAUNCHED_AT: i64 = 1_800_000_000;

#[test]
fn an_agent_execution_whose_worktree_could_not_be_prepared_starts_the_backoff_over() {
    let table = [
        (
            "an unprepared worktree",
            Failure::retryable("worktree_unprepared", String::new()),
        ),
        (
            "a foreign worktree path",
            Failure::not_retryable("worktree_path_foreign", String::new()),
        ),
    ];

    for (case, failure) in table {
        let ending = Ending::Ended(failure.into_end());

        assert_eq!(ending.agent_end(LAUNCHED_AT), AgentEnd::Other, "{case}");
    }
}

#[test]
fn only_a_transient_failure_feeds_the_pause_with_its_reset_and_launch_time() {
    let reset = LAUNCHED_AT + 600;
    let transient = Ending::Ended(ExecutionEnd::Failed {
        class: FailureClass::Transient,
        code: "rate_limit".to_owned(),
        message: String::new(),
        resets_at: Some(reset),
    });
    let otherwise = [
        ("interrupted", Ending::Ended(ExecutionEnd::Interrupted)),
        ("cancelled", Ending::Cancelled),
    ];

    assert_eq!(
        transient.agent_end(LAUNCHED_AT),
        AgentEnd::Transient {
            code: "rate_limit",
            resets_at: Some(reset),
            launched_at: LAUNCHED_AT,
        }
    );
    for (case, ending) in otherwise {
        assert_eq!(ending.agent_end(LAUNCHED_AT), AgentEnd::Other, "{case}");
    }
}
