use super::fixtures::{DATA, Execution, MODEL, REPORT};
use crate::runtime::{Exit, FailureClass, Rejection, Script, StopSignal, Verdict};

fn parsed(text: &str) -> Script {
    Script::from_toml(text).expect("a valid script")
}

async fn verdict(script: Script) -> Verdict {
    let execution = Execution::new();
    let completion = execution
        .complete_script(script, StopSignal::new())
        .await
        .expect("the fake completes");
    execution.verdict(&completion)
}

#[tokio::test]
async fn a_scripted_delivery_writes_its_files_and_reports_its_outcome() {
    let script = parsed(&format!(
        r#"
[[on]]
step = "write"
iteration = 1
attempt = 1
deliver = {{ outcome = "approved", outputs = {{ report = "report.md", data = "out/data.json" }}, files = {{ "report.md" = {REPORT:?}, "out/data.json" = {DATA:?} }} }}
"#
    ));

    let verdict = verdict(script).await;

    let Verdict::Accepted(delivery) = verdict else {
        panic!("expected an accepted delivery, got {verdict:?}");
    };
    assert_eq!(delivery.outcome, "approved");
}

#[tokio::test]
async fn iteration_and_attempt_default_to_the_first_ones() {
    let script = parsed(
        r#"
[[on]]
step = "write"
crash = { exit_code = 2 }
"#,
    );

    let verdict = verdict(script).await;

    assert!(
        matches!(verdict, Verdict::Rejected(Rejection::ExitCode(2))),
        "{verdict:?}"
    );
}

#[tokio::test]
async fn a_later_attempt_is_keyed_by_its_number() {
    let script = parsed(
        r#"
[[on]]
step = "write"
attempt = 2
crash = { exit_code = 2 }
"#,
    );

    let error = Execution::new()
        .complete_script(script, StopSignal::new())
        .await
        .expect_err("attempt 1 is not scripted");

    assert_eq!(error.code, "unscripted");
}

#[tokio::test]
async fn a_model_mismatch_reports_the_other_model() {
    let script = parsed(&format!(
        r#"
[[on]]
step = "write"
model_mismatch = {{ reported = "claude-haiku-5-5", delivery = {{ outcome = "approved", outputs = {{ report = "report.md" }}, files = {{ "report.md" = {REPORT:?} }} }} }}
"#
    ));

    let verdict = verdict(script).await;

    let Verdict::Rejected(rejection) = verdict else {
        panic!("expected a rejection, got {verdict:?}");
    };
    assert_eq!(
        rejection,
        Rejection::ModelMismatch {
            requested: MODEL.to_owned(),
            reported: "claude-haiku-5-5".to_owned()
        }
    );
}

#[tokio::test]
async fn a_limit_failure_carries_its_reset_time() {
    let script = parsed(
        r#"
[[on]]
step = "write"
limit_failure = { resets_at = 1792000000 }
"#,
    );

    let error = Execution::new()
        .complete_script(script, StopSignal::new())
        .await
        .expect_err("a limit failure");

    assert_eq!(error.class, FailureClass::Transient);
    assert_eq!(error.resets_at, Some(1_792_000_000));
}

#[tokio::test]
async fn a_limit_failure_can_reset_some_seconds_after_it_happens() {
    let script = parsed(
        r#"
[[on]]
step = "write"
limit_failure = { resets_in = 30 }
"#,
    );
    let before = jiff::Timestamp::now().as_second();

    let error = Execution::new()
        .complete_script(script, StopSignal::new())
        .await
        .expect_err("a limit failure");

    let after = jiff::Timestamp::now().as_second();
    assert_eq!(error.class, FailureClass::Transient);
    let resets_at = error.resets_at.expect("a reset time");
    assert!(
        (before + 30..=after + 30).contains(&resets_at),
        "{resets_at}"
    );
}

#[tokio::test]
async fn a_final_json_text_replaces_the_envelope() {
    let script = parsed(
        r#"
[[on]]
step = "write"
deliver = { outcome = "approved", final_json = "not json" }
"#,
    );

    let verdict = verdict(script).await;

    assert!(
        matches!(verdict, Verdict::Rejected(Rejection::Delivery(_))),
        "{verdict:?}"
    );
}

#[tokio::test]
async fn waiting_until_stopped_is_a_bare_behaviour() {
    let script = parsed(
        r#"
[[on]]
step = "write"
wait_until_stopped = {}
"#,
    );
    let execution = Execution::new();
    let stop = StopSignal::new();
    stop.graceful();

    let completion = execution
        .complete_script(script, stop)
        .await
        .expect("the fake completes");

    assert_eq!(completion.exit, Exit::Stopped);
}

#[test]
fn an_unknown_behaviour_is_an_invalid_script() {
    let error = Script::from_toml(
        r#"
[[on]]
step = "write"
explode = {}
"#,
    )
    .expect_err("an unknown behaviour");

    assert!(
        error
            .to_string()
            .starts_with("invalid Fake Runtime script: TOML parse error at line 2"),
        "{error}"
    );
}
