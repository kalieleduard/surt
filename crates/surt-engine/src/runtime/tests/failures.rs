use tokio::sync::mpsc;
use uuid::Uuid;

use super::fixtures::{Execution, MODEL, STEP, valid};
use crate::runtime::{
    AgentRuntime, Behavior, Exit, FailureClass, FakeRuntime, FinalResult, Initialization,
    Rejection, RuntimeCompletion, RuntimeEvent, Script, StopLevel, StopSignal, Verdict,
};

const RESET_AT: i64 = 1_792_000_000;
const CRASH_EXIT_CODE: i32 = 137;

fn rejected(verdict: Verdict) -> Rejection {
    let Verdict::Rejected(rejection) = verdict else {
        panic!("expected a rejection, got {verdict:?}");
    };
    rejection
}

#[tokio::test]
async fn a_model_other_than_the_requested_one_is_rejected_and_not_retried() {
    let behavior = Behavior::ModelMismatch {
        reported: "claude-haiku-5-5".to_owned(),
        delivery: valid("approved"),
    };

    let verdict = Execution::new()
        .run(behavior)
        .await
        .expect("the fake completes");

    let rejection = rejected(verdict);
    assert_eq!(
        rejection,
        Rejection::ModelMismatch {
            requested: MODEL.to_owned(),
            reported: "claude-haiku-5-5".to_owned()
        }
    );
    assert_eq!(rejection.class(), FailureClass::NotRetryable);
}

#[tokio::test]
async fn a_crash_is_a_retryable_failure_whatever_it_left_behind() {
    let verdict = Execution::new()
        .run(Behavior::Crash {
            exit_code: CRASH_EXIT_CODE,
        })
        .await
        .expect("the fake completes");

    let rejection = rejected(verdict);
    assert_eq!(rejection, Rejection::ExitCode(CRASH_EXIT_CODE));
    assert_eq!(rejection.class(), FailureClass::Retryable);
}

#[tokio::test]
async fn a_limit_failure_is_transient_and_carries_its_reset_time() {
    for resets_at in [Some(RESET_AT), None] {
        let error = Execution::new()
            .run(Behavior::LimitFailure { resets_at })
            .await
            .expect_err("a limit failure");

        assert_eq!(error.class, FailureClass::Transient);
        assert_eq!(error.resets_at, resets_at);
        assert_eq!(
            error.to_string(),
            "rate_limit: the scripted usage limit was reached"
        );
    }
}

#[tokio::test]
async fn an_unscripted_execution_fails_without_retry() {
    let execution = Execution::new();

    let error = execution
        .complete_script(Script::new(), StopSignal::new())
        .await
        .expect_err("nothing scripted");

    assert_eq!(error.class, FailureClass::NotRetryable);
}

#[tokio::test]
async fn a_waiting_execution_runs_until_it_is_stopped_and_is_then_interrupted() {
    let execution = Execution::new();
    let stop = StopSignal::new();
    let (events, mut received) = mpsc::unbounded_channel();
    let runtime = FakeRuntime::new(Script::new().on(STEP, 1, 1, Behavior::WaitUntilStopped));
    let request = execution.request.clone();
    let waiting = tokio::spawn({
        let stop = stop.clone();
        async move { runtime.execute(request, events, stop).await }
    });

    let launched = received.recv().await;
    let initialized = received.recv().await;
    tokio::task::yield_now().await;
    let still_running = !waiting.is_finished();
    stop.graceful();
    let completion = waiting
        .await
        .expect("the execution task")
        .expect("the fake completes");

    assert_eq!(launched, Some(RuntimeEvent::Launched { pid: None }));
    assert!(matches!(initialized, Some(RuntimeEvent::Initialized(_))));
    assert!(still_running);
    assert_eq!(completion.exit, Exit::Stopped);
    assert!(matches!(
        execution.verdict(&completion),
        Verdict::Interrupted
    ));
}

#[test]
fn a_stop_only_ever_escalates() {
    let stop = StopSignal::new();
    let before = stop.level();

    stop.force();
    stop.graceful();

    assert_eq!(before, StopLevel::None);
    assert_eq!(stop.level(), StopLevel::Force);
}

fn completion(
    exit: Exit,
    initialization: Option<Initialization>,
    result: Option<FinalResult>,
) -> RuntimeCompletion {
    RuntimeCompletion {
        exit,
        initialization,
        result,
    }
}

fn success() -> Option<FinalResult> {
    Some(FinalResult {
        succeeded: true,
        structured_output: Some(r#"{"outcome": "approved", "outputs": {}}"#.to_owned()),
    })
}

#[test]
fn a_completion_without_matching_evidence_is_rejected_before_its_files_are_read() {
    let execution = Execution::new();
    let initialized = |session_id| {
        Some(Initialization {
            session_id,
            model: MODEL.to_owned(),
        })
    };
    let own = initialized(execution.request.session_id);
    let foreign_session = Uuid::now_v7();
    let unsuccessful = Some(FinalResult {
        succeeded: false,
        structured_output: None,
    });
    let cases = [
        (
            completion(Exit::Code(0), None, success()),
            Rejection::MissingInitialization,
        ),
        (
            completion(Exit::Code(0), initialized(foreign_session), success()),
            Rejection::SessionMismatch {
                expected: execution.request.session_id,
                reported: foreign_session,
            },
        ),
        (
            completion(Exit::Code(0), own.clone(), None),
            Rejection::MissingResult,
        ),
        (
            completion(Exit::Code(0), own, unsuccessful),
            Rejection::UnsuccessfulResult,
        ),
    ];

    for (completion, expected) in cases {
        let rejection = rejected(execution.verdict(&completion));

        assert_eq!(rejection, expected);
        assert_eq!(rejection.class(), FailureClass::Retryable);
    }
}
