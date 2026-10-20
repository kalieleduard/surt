use std::num::NonZeroU32;

use serde_json::json;

use super::fixtures::{ACTOR, Feature, Recording, label};
use super::talk::{
    ASKING_LOOP, answer_revisions, answers, ask, input_gate, questions, spec, talk, talk_catalog,
};
use super::team::input;
use crate::project::{Answered, Config, DriveOutcome, Gate, GateKind};
use crate::runtime::Script;

type RevisionRow = (i64, String, Option<String>, Option<String>, String);

fn two_rounds_then_a_spec() -> Script {
    Script::new()
        .on("spec", 1, 1, ask(&["login-id", "lockout"]))
        .on("spec", 2, 1, ask(&["session"]))
        .on("spec", 3, 1, spec("# Login\n"))
}

#[tokio::test(flavor = "multi_thread")]
async fn each_round_of_questions_waits_for_answers_that_accumulate_until_the_spec_is_approved() {
    let feature = talk().await;
    let runtime = Recording::new(two_rounds_then_a_spec());

    let first = feature.drive_with(&runtime).await.outcome;
    let one = answers(&[("login-id", "e-mail only"), ("lockout", "after 5")]);
    let answered = feature
        .project
        .answer(&feature.run, 1, &one, None, ACTOR)
        .await;
    let second = feature.drive_with(&runtime).await.outcome;
    let round = feature
        .project
        .waiting_input(&feature.run)
        .await
        .expect("the second round waits");
    let two = answers(&[("session", "30 days")]);
    feature
        .project
        .answer(&feature.run, 2, &two, Some("decide the rest"), ACTOR)
        .await
        .expect("the second round is answered");
    let third = feature.drive_with(&runtime).await.outcome;
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the spec is approved");
    let last = feature.drive_with(&runtime).await.outcome;

    assert_eq!(first.expect("drive returns"), input_gate(1));
    assert_eq!(
        answered.expect("the first round is answered"),
        Answered {
            step: "answer-spec".to_owned(),
            artifact: "spec-questions".to_owned(),
            revision: 1,
            answer_artifact: "spec-answers".to_owned(),
            answer_revision: 1,
            round: 1,
            actor: ACTOR.to_owned(),
        }
    );
    assert_eq!(second.expect("drive returns"), input_gate(2));
    assert_eq!((round.revision, round.round), (2, 2));
    assert_eq!(
        third.expect("drive returns"),
        DriveOutcome::AtGate {
            gates: vec![Gate {
                step: label("approve-spec", Some(3)),
                kind: GateKind::Approval,
                artifact: "spec".to_owned(),
            }]
        }
    );
    assert_eq!(last.expect("drive returns"), DriveOutcome::Succeeded);
    let requests = runtime.requests();
    assert_eq!(
        input(&requests[0], "spec-answers", "spec-answers.json"),
        None
    );
    assert_eq!(
        input(&requests[2], "spec-answers", "spec-answers.json"),
        Some(json!({ "rounds": [
            { "questions": questions(&["login-id", "lockout"])["questions"],
              "answers": { "login-id": "e-mail only", "lockout": "after 5" } },
            { "questions": questions(&["session"])["questions"],
              "answers": { "session": "30 days" }, "note": "decide the rest" },
        ] }))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_answer_is_a_human_revision_recorded_with_its_events_in_one_step() {
    let feature = talk().await;
    feature
        .drive(Script::new().on("spec", 1, 1, ask(&["login-id"])))
        .await;

    feature
        .project
        .answer(
            &feature.run,
            1,
            &answers(&[("login-id", "e-mail")]),
            None,
            ACTOR,
        )
        .await
        .expect("the round is answered");

    let revisions: Vec<RevisionRow> = feature
        .dir
        .rows(
            "SELECT r.number, r.origin, r.human_action, r.actor, f.rel_path FROM artifact_revision r \
             JOIN artifact a ON a.id = r.artifact_id JOIN revision_file f ON f.revision_id = r.id \
             WHERE a.name = 'spec-answers'",
        )
        .await;
    let step: Vec<(String, Option<String>)> = feature
        .dir
        .rows("SELECT state, outcome FROM step_run WHERE step_key = 'answer-spec'")
        .await;
    let recorded = feature.dir.event_data("answer.recorded").await;
    let done: Vec<(String,)> = feature
        .dir
        .rows(
            "SELECT e.data FROM event e JOIN step_run s ON s.id = e.step_run_id \
             WHERE s.step_key = 'answer-spec' AND e.kind = 'step.done'",
        )
        .await;
    assert_eq!(
        revisions,
        [(
            1,
            "human".to_owned(),
            Some("answer".to_owned()),
            Some(ACTOR.to_owned()),
            "spec-answers.json".to_owned()
        )]
    );
    assert_eq!(step, [("done".to_owned(), Some("answered".to_owned()))]);
    assert_eq!(
        recorded,
        [
            json!({ "artifact": "spec-questions", "revision": 1, "answer_artifact": "spec-answers",
                 "answer_revision": 1, "round": 1, "actor": ACTOR })
        ]
    );
    assert_eq!(done, [(json!({ "outcome": "answered" }).to_string(),)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_delivered_output_skips_the_input_step_and_reaches_the_gate() {
    let feature = talk().await;

    let driven = feature
        .drive(Script::new().on("spec", 1, 1, spec("# Login\n")))
        .await;

    let skipped: Vec<(String,)> = feature
        .dir
        .rows("SELECT state FROM step_run WHERE step_key = 'answer-spec'")
        .await;
    assert!(matches!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::AtGate { gates } if gates[0].kind == GateKind::Approval
    ));
    assert_eq!(skipped, [("skipped".to_owned(),)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_loop_whose_body_holds_an_input_step_asks_past_the_iteration_limit() {
    let config = Config {
        max_iterations: NonZeroU32::new(1).expect("a positive limit"),
        ..Config::default()
    };
    let feature = Feature::with_config(talk_catalog(ASKING_LOOP), "talk", config).await;
    let script = Script::new()
        .on("spec", 1, 1, ask(&["login-id"]))
        .on("spec", 2, 1, ask(&["lockout"]))
        .on("spec", 3, 1, spec("# Login\n"));

    feature.drive(script.clone()).await;
    for (revision, id) in [(1, "login-id"), (2, "lockout")] {
        feature
            .project
            .answer(
                &feature.run,
                revision,
                &answers(&[(id, "yes")]),
                None,
                ACTOR,
            )
            .await
            .expect("the round is answered");
        feature.drive(script.clone()).await;
    }

    let runs: Vec<(String,)> = feature.dir.rows("SELECT state FROM workflow_run").await;
    assert_eq!(runs, [("succeeded".to_owned(),)]);
    assert_eq!(answer_revisions(&feature).await, [(2,)]);
}
