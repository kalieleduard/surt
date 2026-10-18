use serde_json::json;

use super::super::fixtures::SUBJECT;
use super::fixtures::Feature;
use super::fixtures::review_catalog;
use super::inspecting::{rejected, reworked};
use super::repository::git;
use crate::project::{
    AgentProfile, Config, Error, ExecutionCandidate, ExecutionDetail, ExecutionInput,
    ExecutionState, Performer, Reference, Refusal,
};

async fn rejected_once() -> Feature {
    rejected(Feature::on_repository(review_catalog(), "review", Config::default()).await).await
}

const TWIN_SUFFIX: &str = "-7fff-8fff-ffffffffffff";

async fn execution_ids(feature: &Feature, filter: &str) -> Vec<String> {
    let ids: Vec<(String,)> = feature
        .dir
        .rows(&format!(
            "SELECT x.id FROM execution x JOIN step_run s ON s.id = x.step_run_id \
             WHERE {filter} ORDER BY x.started_at, x.rowid"
        ))
        .await;
    ids.into_iter().map(|(id,)| id).collect()
}

async fn detail(feature: &Feature, reference: &str) -> ExecutionDetail {
    feature
        .project
        .inspect_execution(reference)
        .await
        .expect("the execution is inspected")
}

async fn twin_of(feature: &Feature, id: &str) -> String {
    let twin = format!("{}{TWIN_SUFFIX}", &id[..13]);
    feature
        .dir
        .execute(&format!(
            "INSERT INTO execution (id, step_run_id, attempt, kind, agent_snapshot_sha256, \
             state, started_at, finished_at) SELECT '{twin}', step_run_id, attempt + 100, kind, \
             agent_snapshot_sha256, 'interrupted', started_at, finished_at \
             FROM execution WHERE id = '{id}'"
        ))
        .await;
    twin
}

fn candidate_ids(candidates: &[ExecutionCandidate]) -> Vec<&str> {
    candidates
        .iter()
        .map(|candidate| candidate.id.as_str())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_execution_is_found_by_its_full_id_or_a_unique_prefix_of_eight_characters() {
    let feature = rejected_once().await;
    let ids = execution_ids(&feature, "s.step_key = 'write'").await;
    let unique = &ids[1][..ids[1].len() - 1];

    let by_id = detail(&feature, &ids[1]).await;
    let by_prefix = detail(&feature, unique).await;
    let upper = detail(&feature, &ids[1].to_uppercase()).await;

    assert_eq!(by_id.id, ids[1]);
    assert_eq!(by_prefix, by_id);
    assert_eq!(upper, by_id);
    assert_eq!(
        (by_id.subject.as_str(), by_id.run, by_id.item.as_deref()),
        (SUBJECT, 1, None)
    );
    assert_eq!(
        (by_id.step.as_str(), by_id.iteration, by_id.attempt),
        ("write", Some(2), 1)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_detail_shows_the_profile_state_inputs_and_what_the_runtime_reported() {
    let feature = rejected_once().await;
    let ids = execution_ids(&feature, "s.step_key = 'write'").await;

    let shown = detail(&feature, &ids[1]).await;

    assert_eq!(
        shown.performer,
        Performer::Agent {
            role: "writer".to_owned(),
            profile: AgentProfile {
                runtime: "fake".to_owned(),
                model: "fake-model".to_owned(),
                effort: "high".to_owned(),
                permission_mode: None,
            },
        }
    );
    assert_eq!(
        (shown.state, shown.outcome.as_deref()),
        (ExecutionState::Succeeded, Some("done"))
    );
    assert_eq!(shown.failure, None);
    assert!(shown.started_at.is_some() && shown.finished_at >= shown.started_at);
    let inputs: Vec<(&str, &str, u32)> = shown
        .inputs
        .iter()
        .map(|input| (input.name.as_str(), input.artifact.as_str(), input.revision))
        .collect();
    assert_eq!(
        inputs,
        [
            ("draft", "draft", 1),
            ("draft-rejection", "draft-rejection", 1),
            ("request", "request", 1)
        ]
    );
    assert!(shown.inputs.iter().all(|input| input.sha256.len() == 64));
    assert_eq!(shown.reported.model.as_deref(), Some("fake-model"));
    assert_eq!(shown.reported.exit_code, Some(0));
    assert!(shown.reported.session_id.is_some());
    assert_eq!(
        (shown.reported.turns, shown.reported.cost_usd),
        (None, None)
    );
    assert_eq!(shown.reported.usage, None);
    assert_eq!((shown.worktree, shown.salvage_ref), (None, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn reported_usage_cost_and_denials_are_read_and_absent_values_stay_absent() {
    let feature = rejected_once().await;
    let ids = execution_ids(&feature, "s.step_key = 'write'").await;
    let info = json!({
        "runtime_version": "2.1.0",
        "turns": 4,
        "cost_usd": 0.25,
        "usage": { "input_tokens": 1200, "output_tokens": 300 },
        "permission_denials": [{ "tool_name": "Write" }],
    });
    feature
        .dir
        .execute(&format!(
            "UPDATE execution SET runtime_info = '{info}' WHERE id = '{}'",
            ids[0]
        ))
        .await;

    let shown = detail(&feature, &ids[0]).await;

    let reported = shown.reported;
    assert_eq!(reported.version.as_deref(), Some("2.1.0"));
    assert_eq!((reported.turns, reported.cost_usd), (Some(4), Some(0.25)));
    let usage = reported.usage.expect("the usage is reported");
    assert_eq!(
        (usage.input_tokens, usage.output_tokens),
        (Some(1200), Some(300))
    );
    assert_eq!(
        (
            usage.cache_creation_input_tokens,
            usage.cache_read_input_tokens
        ),
        (None, None)
    );
    assert_eq!(
        reported.permission_denials,
        vec![json!({ "tool_name": "Write" })]
    );
    assert_eq!((reported.session_id, reported.exit_code), (None, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tasks_execution_names_its_task_worktree_and_item_input() {
    let feature = reworked().await;
    let ids = execution_ids(&feature, "s.step_key = 'build' AND s.iteration = 2").await;

    let shown = detail(&feature, &ids[0]).await;

    assert_eq!(shown.item.as_deref(), Some("A"));
    assert_eq!(
        (shown.step.as_str(), shown.iteration, shown.attempt),
        ("build", Some(2), 1)
    );
    let worktree = shown.worktree.expect("the task ran in a worktree");
    assert!(worktree.ends_with("A"), "{}", worktree.display());
    let item: Vec<&ExecutionInput> = shown
        .inputs
        .iter()
        .filter(|input| input.name == "item")
        .collect();
    assert_eq!(item.len(), 1);
    assert_eq!(
        (item[0].artifact.as_str(), item[0].item.as_deref()),
        ("plan", Some("A"))
    );
    assert!(shown.inputs.iter().any(|input| input.name == "feedback"));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_operation_execution_names_its_operation() {
    let feature = reworked().await;
    let ids = execution_ids(&feature, "s.step_key = 'check'").await;

    let shown = detail(&feature, &ids[0]).await;

    assert_eq!(
        shown.performer,
        Performer::Operation {
            id: "checks.run".to_owned()
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_prefix_shared_by_several_executions_refuses_and_lists_them() {
    let feature = rejected_once().await;
    let ids = execution_ids(&feature, "s.step_key = 'write'").await;
    let all = execution_ids(&feature, "1 = 1").await;
    let twin = twin_of(&feature, &ids[0]).await;

    let refused = feature.project.inspect_execution(&ids[0][..13]).await;

    match refused {
        Err(Error::Refused(Refusal::AmbiguousExecution {
            reference,
            candidates,
        })) => {
            assert_eq!(reference, ids[0][..13]);
            let mut expected: Vec<&str> = all
                .iter()
                .map(String::as_str)
                .filter(|id| id.starts_with(&ids[0][..13]))
                .chain([twin.as_str()])
                .collect();
            expected.sort_unstable();
            assert_eq!(candidate_ids(&candidates), expected);
            let first = candidates
                .iter()
                .find(|candidate| candidate.id == ids[0])
                .expect("the original is a candidate");
            assert_eq!(
                (first.subject.as_str(), first.run, first.step.as_str()),
                (SUBJECT, 1, "write")
            );
            assert_eq!((first.iteration, first.attempt), (Some(1), 1));
            assert_eq!(first.state, ExecutionState::Succeeded);
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_prefix_shorter_than_eight_characters_refuses_and_lists_the_matches() {
    let feature = rejected_once().await;
    let ids = execution_ids(&feature, "1 = 1").await;
    let prefix = &ids[0][..7];

    let refused = feature.project.inspect_execution(prefix).await;

    match refused {
        Err(Error::Refused(Refusal::ExecutionPrefixTooShort {
            reference,
            minimum,
            candidates,
        })) => {
            assert_eq!((reference.as_str(), minimum), (prefix, 8));
            assert!(candidate_ids(&candidates).contains(&ids[0].as_str()));
            assert!(
                candidates
                    .iter()
                    .all(|candidate| candidate.id.starts_with(prefix))
            );
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_id_no_execution_has_is_unknown() {
    let feature = rejected_once().await;

    let refused = feature.project.inspect_execution("ffffffff-0000").await;

    assert!(
        matches!(
            &refused,
            Err(Error::Refused(Refusal::UnknownExecution { reference })) if reference == "ffffffff-0000"
        ),
        "{refused:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_reference_names_a_feature_first_and_otherwise_an_execution() {
    let feature = rejected_once().await;
    let ids = execution_ids(&feature, "1 = 1").await;

    let subject = feature.project.reference(SUBJECT).await;
    let execution = feature.project.reference(&ids[0]).await;
    let neither = feature.project.reference("no-such-thing").await;

    assert!(
        matches!(&subject, Ok(Reference::Subject(found)) if found.name == SUBJECT),
        "{subject:?}"
    );
    assert!(
        matches!(&execution, Ok(Reference::Execution(id)) if *id == ids[0]),
        "{execution:?}"
    );
    assert!(
        matches!(neither, Err(Error::Refused(Refusal::UnknownSubject { .. }))),
        "{neither:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_salvage_ref_named_after_the_execution_is_shown() {
    let feature = rejected_once().await;
    let ids = execution_ids(&feature, "s.step_key = 'write'").await;
    let salvage = format!("refs/surt/salvage/{}", ids[0]);
    let other = format!("refs/surt/salvage/{}", ids[1]);
    git(&feature.dir.path(), &["update-ref", &salvage, "HEAD"]);
    git(&feature.dir.path(), &["update-ref", &other, "HEAD"]);

    let shown = detail(&feature, &ids[0]).await;

    assert_eq!(shown.salvage_ref, Some(salvage));
}
