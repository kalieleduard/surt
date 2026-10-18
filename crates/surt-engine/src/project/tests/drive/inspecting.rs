use serde_json::{Map, Value, json};

use std::path::Path;

use super::super::fixtures::SUBJECT;
use super::evidence::change;
use super::fixtures::{ACTOR, Feature, draft};
use super::repository::commit_file;
use super::team::{REVIEWED_TASK, Team, blocking, builds, built, reviews, reviews_with};
use crate::project::{
    Config, Error, InspectRequest, Refusal, RunSelector, Timeline, TimelineEvent,
};
use crate::runtime::Script;

fn whole_run() -> InspectRequest<'static> {
    InspectRequest {
        item: None,
        run: RunSelector::Current,
    }
}

fn task(key: &str) -> InspectRequest<'_> {
    InspectRequest {
        item: Some(key),
        run: RunSelector::Current,
    }
}

fn kinds(timeline: &Timeline) -> Vec<&str> {
    timeline
        .events
        .iter()
        .map(|event| event.kind.as_str())
        .collect()
}

fn of_kind<'a>(timeline: &'a Timeline, kind: &str) -> Vec<&'a TimelineEvent> {
    timeline
        .events
        .iter()
        .filter(|event| event.kind == kind)
        .collect()
}

fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

async fn rejected_once() -> Feature {
    rejected(Feature::review().await).await
}

pub(super) async fn rejected(feature: Feature) -> Feature {
    let script =
        Script::new()
            .on("write", 1, 1, draft("# Draft\n"))
            .on("write", 2, 1, draft("# Draft 2\n"));
    feature.drive(script.clone()).await;
    feature
        .project
        .reject(&feature.run, Some(1), "Too short.\nAdd errors.", ACTOR)
        .await
        .expect("the draft is rejected");
    feature.drive(script).await;
    feature
}

fn rework(worktree: &Path) -> String {
    commit_file(worktree, "feature.txt", "the reworked feature\n")
}

pub(super) async fn reworked() -> Feature {
    let feature = built(REVIEWED_TASK, &[("A", &[])], Config::default()).await;
    let team = Team::new()
        .on("A/build", 1, 1, builds(change))
        .on(
            "A/review",
            1,
            1,
            reviews_with("changes_requested", vec![blocking("feature.txt")]),
        )
        .on("A/build", 2, 1, builds(rework))
        .on("A/review", 2, 1, reviews("approved"));
    feature.drive_with(&team).await;
    feature
}

#[tokio::test(flavor = "multi_thread")]
async fn the_run_timeline_lists_the_runs_events_in_order_with_their_details() {
    let feature = rejected_once().await;

    let timeline = feature
        .project
        .inspect(SUBJECT, whole_run())
        .await
        .expect("the run is inspected");

    assert_eq!((timeline.subject.as_str(), timeline.run), (SUBJECT, 1));
    assert_eq!(timeline.item, None);
    let listed = kinds(&timeline);
    assert_eq!(listed.first(), Some(&"run.created"));
    assert_eq!(
        listed
            .iter()
            .filter(|kind| **kind == "driver.started")
            .count(),
        2
    );
    let approvals = of_kind(&timeline, "approval.recorded");
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].step.as_deref(), Some("approve"));
    assert_eq!(approvals[0].iteration, Some(1));
    assert_eq!(approvals[0].data["reason"], json!("Too short."));
    let started = of_kind(&timeline, "execution.started");
    assert_eq!(
        started
            .iter()
            .map(|event| (event.step.as_deref(), event.iteration, event.attempt))
            .collect::<Vec<_>>(),
        [
            (Some("write"), Some(1), Some(1)),
            (Some("write"), Some(2), Some(1))
        ]
    );
    assert!(started.iter().all(|event| event.execution.is_some()));
    assert!(
        timeline
            .events
            .windows(2)
            .all(|pair| pair[0].at <= pair[1].at)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_task_timeline_explains_a_new_iteration_after_a_review_asked_for_changes() {
    let feature = reworked().await;

    let timeline = feature
        .project
        .inspect(SUBJECT, task("A"))
        .await
        .expect("the task is inspected");

    assert_eq!(timeline.item.as_deref(), Some("A"));
    assert!(
        timeline
            .events
            .iter()
            .all(|event| event.item.as_deref() == Some("A"))
    );
    assert_eq!(kinds(&timeline).first(), Some(&"run.created"));
    let iterations: Vec<_> = of_kind(&timeline, "iteration.started")
        .into_iter()
        .map(|event| event.data.clone())
        .collect();
    assert_eq!(
        iterations,
        [
            object(json!({ "iteration": 1, "cause": "loop_started" })),
            object(json!({
                "iteration": 2,
                "cause": "until_not_met",
                "step": "review",
                "outcome": "changes_requested"
            })),
        ]
    );
    let builds: Vec<_> = of_kind(&timeline, "execution.started")
        .into_iter()
        .filter(|event| event.step.as_deref() == Some("build"))
        .map(|event| event.iteration)
        .collect();
    assert_eq!(builds, [Some(1), Some(2)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_run_timeline_shows_when_each_task_was_created_but_not_its_steps() {
    let feature = reworked().await;

    let timeline = feature
        .project
        .inspect(SUBJECT, whole_run())
        .await
        .expect("the run is inspected");

    let created: Vec<_> = of_kind(&timeline, "run.created")
        .into_iter()
        .map(|event| event.item.as_deref())
        .collect();
    assert_eq!(created, [None, Some("A")]);
    assert!(
        timeline
            .events
            .iter()
            .all(|event| event.item.is_none() || event.kind.starts_with("run."))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_task_refuses_and_lists_the_runs_tasks() {
    let feature = reworked().await;

    let refused = feature.project.inspect(SUBJECT, task("Z")).await;

    match refused {
        Err(Error::Refused(Refusal::UnknownItem { item, known, .. })) => {
            assert_eq!((item.as_str(), known), ("Z", vec!["A".to_owned()]));
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn events_without_details_still_render_from_their_kinds_and_times() {
    let feature = rejected_once().await;
    let detailed = feature
        .project
        .inspect(SUBJECT, whole_run())
        .await
        .expect("the run is inspected");
    feature.dir.execute("DROP TRIGGER event_append_only").await;
    feature.dir.execute("UPDATE event SET data = '{}'").await;

    let bare = feature
        .project
        .inspect(SUBJECT, whole_run())
        .await
        .expect("the run is inspected");

    assert_eq!(kinds(&bare), kinds(&detailed));
    assert!(bare.events.iter().all(|event| event.data.is_empty()));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_run_ordinal_refuses() {
    let feature = rejected_once().await;
    let request = InspectRequest {
        item: None,
        run: RunSelector::Ordinal(2),
    };

    let refused = feature.project.inspect(SUBJECT, request).await;

    assert!(
        matches!(
            refused,
            Err(Error::Refused(Refusal::UnknownRun { run: 2, .. }))
        ),
        "{refused:?}"
    );
}
