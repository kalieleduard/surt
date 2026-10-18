use serde_json::{Value, json};

use super::fixtures::{Feature, Recording, draft};
use super::stopping::{Interference, Interfering};
use crate::runtime::{Behavior, Script, StopLevel};

const CRASH_EXIT_CODE: i32 = 9;
const RESETS_AT: i64 = 1_900_000_000;
const MODEL: &str = "fake-model";

async fn runtime_infos(feature: &Feature) -> Vec<Value> {
    let infos: Vec<(Option<String>,)> = feature
        .dir
        .rows("SELECT runtime_info FROM execution ORDER BY attempt")
        .await;
    infos
        .into_iter()
        .map(|(info,)| {
            info.map_or(Value::Null, |info| {
                serde_json::from_str(&info).expect("runtime_info is JSON")
            })
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn each_execution_records_what_its_runtime_reported() {
    let feature = Feature::review().await;
    let crash = Behavior::Crash {
        exit_code: CRASH_EXIT_CODE,
    };
    let runtime = Recording::new(Script::new().on("write", 1, 1, crash).on(
        "write",
        1,
        2,
        draft("first draft"),
    ));

    feature.drive_with(&runtime).await;

    let sessions: Vec<String> = runtime
        .requests()
        .iter()
        .map(|request| request.session_id.to_string())
        .collect();
    assert_eq!(
        runtime_infos(&feature).await,
        [
            json!({ "session_id": sessions[0], "model": MODEL, "exit_code": CRASH_EXIT_CODE }),
            json!({ "session_id": sessions[1], "model": MODEL, "exit_code": 0 }),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_execution_keeps_its_report_beside_the_reset_time() {
    let feature = Feature::review().await;
    let limited = Behavior::LimitFailure {
        resets_at: Some(RESETS_AT),
    };
    let runtime = Recording::new(Script::new().on("write", 1, 1, limited));

    feature.drive_with(&runtime).await;

    let session = runtime.requests()[0].session_id.to_string();
    assert_eq!(
        runtime_infos(&feature).await,
        [json!({ "session_id": session, "model": MODEL, "resets_at": RESETS_AT })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interrupted_execution_records_what_was_known_before_it_ended() {
    let feature = Feature::review().await;
    let runtime = Interfering::new(
        Script::new().on("write", 1, 1, Behavior::WaitUntilStopped),
        "write",
        Interference::Stop(StopLevel::Graceful),
    );

    runtime.drive(&feature).await;

    let infos = runtime_infos(&feature).await;
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0]["model"], MODEL);
    assert!(infos[0]["session_id"].is_string(), "{}", infos[0]);
    assert_eq!(infos[0].get("exit_code"), None);
}
