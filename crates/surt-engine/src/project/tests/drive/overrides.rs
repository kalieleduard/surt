use std::num::NonZeroU32;

use super::super::fixtures::{ProjectDir, default_config};
use super::fixtures::{ACTOR, Feature, Recording, draft};
use super::team::failures;
use crate::project::{AgentAssets, Catalog, DriveOutcome};
use crate::runtime::{Behavior, Delivery, RuntimeRequest, Script};

const CRASH_EXIT_CODE: i32 = 9;

const LOCAL_WRITER: &str = r#"
[profile]
model = "fake-model-large"
effort = "low"

[profile.limits]
max_turns = 12
"#;

const EDITED_WRITER: &str = r#"
[profile]
model = "fake-model-edited"
"#;

fn profile(request: &RuntimeRequest) -> (String, String, Option<NonZeroU32>) {
    let profile = &request.launch.profile;
    (
        profile.model.clone(),
        format!("{:?}", profile.effort),
        profile.limits.and_then(|limits| limits.max_turns),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn every_execution_of_a_run_keeps_the_override_it_froze_after_the_file_changes() {
    let feature = Feature::review_with_local("writer", "agent.toml", LOCAL_WRITER).await;
    let crash = Behavior::Crash {
        exit_code: CRASH_EXIT_CODE,
    };
    let first = Recording::new(Script::new().on("write", 1, 1, crash).on(
        "write",
        1,
        2,
        draft("# Draft 1\n"),
    ));
    feature
        .drive_with(&first)
        .await
        .outcome
        .expect("the first draft reaches the gate");
    feature
        .dir
        .write_local("writer", "agent.toml", EDITED_WRITER);
    feature
        .project
        .reject(&feature.run, Some(1), "shorter", ACTOR)
        .await
        .expect("the rejection is recorded");
    let second = Recording::new(Script::new().on("write", 2, 1, draft("# Draft 2\n")));

    feature
        .drive_with(&second)
        .await
        .outcome
        .expect("the second draft reaches the gate");

    let frozen = (
        "fake-model-large".to_owned(),
        "Low".to_owned(),
        NonZeroU32::new(12),
    );
    let profiles: Vec<_> = first
        .requests()
        .iter()
        .chain(second.requests().iter())
        .map(profile)
        .collect();
    assert_eq!(profiles, [frozen.clone(), frozen.clone(), frozen]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_runtime_receives_the_local_instructions_in_place_of_the_built_in_text() {
    let local = "Follow the project's style guide.\n";
    let feature = Feature::review_with_local("writer", "instructions.md", local).await;
    let recording = Recording::new(Script::new().on("write", 1, 1, draft("# Draft\n")));

    feature
        .drive_with(&recording)
        .await
        .outcome
        .expect("the draft reaches the gate");

    let instructions: Vec<String> = recording
        .requests()
        .iter()
        .map(|request| request.launch.instructions.clone())
        .collect();
    assert_eq!(instructions, [local]);
}

const NOTED_WORKFLOW: &str = r#"
schema = 1
name = "noted"

[steps.note]
kind = "agent"
agent = "noter"
"#;

const NOTER: &str = r#"
schema = 1
id = "noter"

[outputs.notes]
format = "json"
json_schema = "schemas/notes.schema.json"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const NOTES: &str = "schemas/notes.schema.json";

const STRICT_NOTES: &str = r#"{"type": "object", "required": ["status"]}"#;

fn noted_catalog() -> Catalog {
    Catalog::new().workflow("noted", NOTED_WORKFLOW).agent(
        "noter",
        AgentAssets {
            toml: NOTER,
            instructions: "Take notes.\n",
            schemas: &[(NOTES, r#"{"type": "object"}"#)],
        },
    )
}

fn notes(text: &str) -> Behavior {
    Behavior::Deliver(Delivery::new("done").output("notes", "notes.json", text))
}

async fn noted_with_local(files: &[(&str, &str)]) -> Feature {
    let dir = ProjectDir::new();
    for (rel_path, text) in files {
        dir.write_local("noter", rel_path, text);
    }
    Feature::in_dir(dir, noted_catalog(), "noted", default_config()).await
}

async fn rejected_then_accepted(feature: &Feature, violating: &str, satisfying: &str) {
    let script =
        Script::new()
            .on("note", 1, 1, notes(violating))
            .on("note", 1, 2, notes(satisfying));

    let driven = feature.drive(script).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let refused: Vec<_> = failures(feature, "note")
        .await
        .into_iter()
        .map(|(iteration, attempt, code, _)| (iteration, attempt, code))
        .collect();
    assert_eq!(refused, [(0, 1, "invalid_delivery".to_owned())]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_delivery_is_validated_against_the_local_schema_that_overlays_the_built_in() {
    let feature = noted_with_local(&[(NOTES, STRICT_NOTES)]).await;

    rejected_then_accepted(&feature, "{}", r#"{"status": "ok"}"#).await;
}

const SPLIT_NOTES: &str = r#"{"type": "object", "properties": {"status": {"$ref": "defs/status.schema.json#/$defs/status"}}}"#;

const STATUS_DEFS: &str = r#"{"$defs": {"status": {"enum": ["ok", "late"]}}}"#;

#[tokio::test(flavor = "multi_thread")]
async fn a_delivery_is_validated_against_a_local_schema_split_with_a_local_ref() {
    let feature = noted_with_local(&[
        (NOTES, SPLIT_NOTES),
        ("schemas/defs/status.schema.json", STATUS_DEFS),
    ])
    .await;

    rejected_then_accepted(&feature, r#"{"status": "lost"}"#, r#"{"status": "late"}"#).await;
}
