use super::super::fixtures::SUBJECT;
use super::super::refusal;
use super::fixtures::{ACTOR, Feature};
use super::talk::{answer_revisions, answers, ask, input_gate, spec, talk, undriven};
use crate::project::{GateKind, GateStatus, Refusal, RunSelector, RunState, StatusScope};
use crate::runtime::Script;

async fn asked() -> Feature {
    let feature = talk().await;
    let driven = feature
        .drive(Script::new().on("spec", 1, 1, ask(&["login-id", "lockout"])))
        .await;
    assert_eq!(driven.outcome.expect("drive returns"), input_gate(1));
    undriven(&feature).await;
    feature
}

#[tokio::test(flavor = "multi_thread")]
async fn answering_refuses_stale_incomplete_or_unknown_answers_and_records_nothing() {
    let feature = asked().await;
    let complete = answers(&[("login-id", "e-mail"), ("lockout", "never")]);
    let partial = answers(&[("login-id", "e-mail"), ("typo", "?")]);

    let stale = feature
        .project
        .answer(&feature.run, 2, &complete, None, ACTOR)
        .await;
    let unanswered = feature
        .project
        .answer(&feature.run, 1, &partial, None, ACTOR)
        .await;

    assert_eq!(
        refusal(stale),
        Refusal::StaleRevision {
            subject: SUBJECT.to_owned(),
            artifact: "spec-questions".to_owned(),
            expected: 2,
            latest: 1,
        }
    );
    assert_eq!(
        refusal(unanswered),
        Refusal::UnansweredQuestions {
            subject: SUBJECT.to_owned(),
            artifact: "spec-questions".to_owned(),
            revision: 1,
            missing: vec!["lockout".to_owned()],
            unknown: vec!["typo".to_owned()],
        }
    );
    assert_eq!(answer_revisions(&feature).await, [(0,)]);
    let waiting = feature
        .project
        .waiting_input(&feature.run)
        .await
        .expect("the round still waits");
    assert_eq!((waiting.revision, waiting.round), (1, 1));
    let ids: Vec<&str> = waiting
        .questions
        .questions()
        .iter()
        .map(|question| question.id())
        .collect();
    assert_eq!(ids, ["login-id", "lockout"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn answering_refuses_when_no_input_step_waits_naming_what_the_run_waits_for() {
    let feature = talk().await;
    feature
        .drive(Script::new().on("spec", 1, 1, spec("# Login\n")))
        .await;
    undriven(&feature).await;

    let early = feature.project.waiting_input(&feature.run).await;
    let answered = feature
        .project
        .answer(&feature.run, 1, &answers(&[]), None, ACTOR)
        .await;

    let expected = Refusal::NoInputWaiting {
        subject: SUBJECT.to_owned(),
        states: vec![RunState::AwaitingApproval],
    };
    assert_eq!(refusal(early), expected);
    assert_eq!(refusal(answered), expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn approve_and_reject_refuse_while_only_an_input_step_waits() {
    let feature = asked().await;

    let approved = feature.project.approve(&feature.run, None, ACTOR).await;
    let rejected = feature
        .project
        .reject(&feature.run, None, "no", ACTOR)
        .await;

    let expected = Refusal::NoGateWaiting {
        subject: SUBJECT.to_owned(),
        input: Some("answer-spec".to_owned()),
    };
    assert_eq!(refusal(approved), expected);
    assert_eq!(refusal(rejected), expected);
    let decisions: Vec<(i64,)> = feature.dir.rows("SELECT count(*) FROM approval").await;
    assert_eq!(decisions, [(0,)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn status_shows_the_run_awaiting_an_answer_with_the_round_then_who_answered() {
    let feature = asked().await;
    let scope = StatusScope::Subject(SUBJECT, RunSelector::Current);

    let waiting = feature.project.status(scope).await.expect("the status");
    feature
        .project
        .answer(
            &feature.run,
            1,
            &answers(&[("login-id", "e-mail"), ("lockout", "never")]),
            None,
            ACTOR,
        )
        .await
        .expect("the round is answered");
    let answered = feature.project.status(scope).await.expect("the status");
    let gate = feature
        .project
        .gate_status(&feature.run, "answer-spec")
        .await
        .expect("the input step's status");

    assert_eq!(waiting[0].states, [RunState::AwaitingAnswer]);
    let view = &waiting[0].gates[0];
    assert_eq!(
        (view.kind, view.artifact.as_str(), view.status, view.round),
        (
            GateKind::Input,
            "spec-questions",
            GateStatus::Waiting { revision: 1 },
            Some(1)
        )
    );
    assert_eq!(answered[0].states, [RunState::Interrupted]);
    let view = &answered[0].gates[0];
    assert_eq!(
        (view.status, view.round, view.decided_by.as_deref()),
        (GateStatus::Answered { revision: 1 }, Some(1), Some(ACTOR))
    );
    assert_eq!(gate, GateStatus::Answered { revision: 1 });
}
