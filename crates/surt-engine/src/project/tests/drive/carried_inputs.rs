use std::fs;

use super::super::fixtures::{REQUEST, SUBJECT, git, input};
use super::fixtures::{ACTOR, Feature, REVIEW_WORKFLOW};
use super::working_copy::at_gate;
use crate::project::{Carry, Error, InputProblem, NewRun, Refusal};

const CHANGED_REQUEST: &str = "add login with sso";

fn next_run(inputs: &[(&str, &str)]) -> NewRun {
    NewRun {
        subject: SUBJECT.to_owned(),
        workflow: "review".to_owned(),
        inputs: inputs
            .iter()
            .map(|(name, value)| input(name, value))
            .collect(),
        carry: Carry::PreviousInputs,
        merge_into_base: None,
        actor: ACTOR.to_owned(),
    }
}

async fn ended() -> Feature {
    let feature = at_gate().await;
    feature.cancel().await.outcome.expect("run 1 is cancelled");
    feature
}

async fn input_texts(feature: &Feature, name: &str) -> Vec<String> {
    let rows: Vec<(String,)> = feature
        .dir
        .rows(&format!(
            "SELECT CAST(b.bytes AS TEXT) FROM artifact a \
             JOIN artifact_revision r ON r.artifact_id = a.id \
             JOIN revision_file f ON f.revision_id = r.id JOIN blob b ON b.sha256 = f.blob_sha256 \
             WHERE a.name = '{name}' ORDER BY r.rowid"
        ))
        .await;
    rows.into_iter().map(|(text,)| text).collect()
}

fn commit_on_main(feature: &Feature) {
    git(
        &feature.dir.path(),
        &[
            "-c",
            "user.name=Surt Tester",
            "-c",
            "user.email=tester@example.com",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "moved on",
        ],
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_next_run_carries_the_text_inputs_and_resolves_its_git_refs_again() {
    let feature = ended().await;
    let first_base = feature.dir.head();
    commit_on_main(&feature);

    let created = feature
        .project
        .create_run(next_run(&[]))
        .await
        .expect("run 2 is created");

    let moved = feature.dir.head();
    assert_ne!(first_base, moved);
    assert_eq!(input_texts(&feature, "request").await, [REQUEST, REQUEST]);
    let bases = input_texts(&feature, "base").await;
    assert!(bases[0].contains(&first_base), "{bases:?}");
    assert!(bases[1].contains(&moved), "{bases:?}");
    assert_eq!(created.git_refs.len(), 1);
    assert_eq!(created.git_refs[0].1.sha, moved);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_given_input_replaces_the_carried_one() {
    let feature = ended().await;

    feature
        .project
        .create_run(next_run(&[("request", CHANGED_REQUEST)]))
        .await
        .expect("run 2 is created");

    assert_eq!(
        input_texts(&feature, "request").await,
        [REQUEST, CHANGED_REQUEST]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_input_the_previous_run_did_not_have_must_be_given() {
    let feature = ended().await;
    let workflow = REVIEW_WORKFLOW.replace(
        "[steps.write]",
        "[inputs.audience]\nformat = \"text\"\n\n[steps.write]",
    );
    fs::write(
        feature.dir.path().join(".surt/workflows/review.toml"),
        workflow,
    )
    .expect("the workflow is edited");

    let missing = feature.project.create_run(next_run(&[])).await;
    let given = feature
        .project
        .create_run(next_run(&[("audience", "admins")]))
        .await;

    assert!(
        matches!(
            &missing,
            Err(Error::Refused(Refusal::InvalidInput {
                problem: InputProblem::Missing { name },
                ..
            })) if name == "audience"
        ),
        "{missing:?}"
    );
    assert!(given.is_ok(), "{given:?}");
    assert_eq!(input_texts(&feature, "audience").await, ["admins"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_latest_workflow_names_the_workflow_of_the_subjects_last_run() {
    let feature = Feature::review().await;

    let latest = feature.project.latest_workflow(SUBJECT).await;
    let unknown = feature.project.latest_workflow("20261017-elsewhere").await;

    assert_eq!(latest.expect("the store reads"), Some("review".to_owned()));
    assert_eq!(unknown.expect("the store reads"), None);
}
