use std::num::NonZeroU32;

use super::fixtures::{ACTOR, Feature, Recording, draft};
use crate::runtime::{Behavior, RuntimeRequest, Script};

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
