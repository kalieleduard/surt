use std::fs;

use super::super::fixtures::SUBJECT;
use super::checks::{check, with_checks};
use super::fixtures::Feature;
use super::integration::independent_tasks;
use super::team::{INTEGRATED_TASK, built};
use crate::project::{Error, LogRequest, LogSource, Refusal, RunSelector};

fn whole_run() -> LogRequest<'static> {
    LogRequest {
        item: None,
        run: RunSelector::Current,
    }
}

fn task(key: &str) -> LogRequest<'_> {
    LogRequest {
        item: Some(key),
        run: RunSelector::Current,
    }
}

async fn delivered_with_one_recheck() -> Feature {
    let config = with_checks(vec![check("head", &["git", "rev-parse", "HEAD"])]);
    let feature = built(INTEGRATED_TASK, &[("A", &[]), ("B", &[])], config).await;
    feature.drive_with(&independent_tasks()).await;
    feature
}

async fn rechecked_task(feature: &Feature) -> String {
    let integrations: Vec<(String, String)> = feature
        .dir
        .rows(
            "SELECT w.item_key, e.id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id WHERE s.step_key = 'integrate'",
        )
        .await;
    integrations
        .into_iter()
        .find(|(_, execution)| feature.integration_log(execution).is_some())
        .map(|(key, _)| key)
        .expect("one integration re-ran the checks")
}

async fn execution_of(feature: &Feature, item: &str, step: &str) -> String {
    let ids: Vec<(String,)> = feature
        .dir
        .rows(&format!(
            "SELECT e.id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.item_key IS '{item}' AND s.step_key = '{step}'"
        ))
        .await;
    ids.into_iter()
        .map(|(id,)| id)
        .next()
        .expect("the execution exists")
}

fn scopes(sources: &[LogSource]) -> Vec<(Option<&str>, &str, u32)> {
    sources
        .iter()
        .map(|source| (source.item.as_deref(), source.step.as_str(), source.attempt))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_feature_logs_the_latest_execution_with_a_log_of_the_run_and_of_each_task() {
    let feature = delivered_with_one_recheck().await;
    let rechecked = rechecked_task(&feature).await;
    let step_of = |key: &str| {
        if key == rechecked {
            "integrate"
        } else {
            "review"
        }
    };

    let logs = feature
        .project
        .logs(SUBJECT, whole_run())
        .await
        .expect("the logs are selected");

    assert_eq!((logs.subject.as_str(), logs.run), (SUBJECT, 1));
    assert_eq!(
        scopes(&logs.sources),
        [
            (None, "plan", 1),
            (Some("A"), step_of("A"), 1),
            (Some("B"), step_of("B"), 1)
        ]
    );
    for source in &logs.sources {
        assert_eq!(
            source.directory,
            feature
                .root()
                .join(".surt/logs")
                .join(SUBJECT)
                .join(&source.execution)
        );
        assert!(source.directory.is_dir(), "{source:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_task_lists_every_execution_with_a_log_in_start_order() {
    let feature = delivered_with_one_recheck().await;
    let rechecked = rechecked_task(&feature).await;

    let logs = feature
        .project
        .logs(SUBJECT, task(&rechecked))
        .await
        .expect("the task's logs are selected");

    assert_eq!(
        scopes(&logs.executions),
        [
            (Some(rechecked.as_str()), "build", 1),
            (Some(rechecked.as_str()), "check", 1),
            (Some(rechecked.as_str()), "review", 1),
            (Some(rechecked.as_str()), "integrate", 1)
        ]
    );
    assert_eq!(logs.executions.last(), logs.sources.last());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_task_narrows_the_logs_to_its_latest_execution_even_when_its_log_was_removed() {
    let feature = delivered_with_one_recheck().await;
    let other = if rechecked_task(&feature).await == "A" {
        "B"
    } else {
        "A"
    };
    let review = execution_of(&feature, other, "review").await;
    fs::remove_dir_all(
        feature
            .root()
            .join(".surt/logs")
            .join(SUBJECT)
            .join(&review),
    )
    .expect("the log is removed");

    let logs = feature
        .project
        .logs(SUBJECT, task(other))
        .await
        .expect("the task's logs are selected");

    assert_eq!(scopes(&logs.sources), [(Some(other), "review", 1)]);
    assert_eq!(logs.sources[0].execution, review);
    assert!(!logs.sources[0].directory.exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_task_or_run_refuses() {
    let feature = delivered_with_one_recheck().await;

    let task = feature.project.logs(SUBJECT, task("Z")).await;
    let run = feature
        .project
        .logs(
            SUBJECT,
            LogRequest {
                item: None,
                run: RunSelector::Ordinal(2),
            },
        )
        .await;

    assert!(
        matches!(
            &task,
            Err(Error::Refused(Refusal::UnknownItem { item, known, .. }))
                if item == "Z" && known == &["A", "B"]
        ),
        "{task:?}"
    );
    assert!(
        matches!(
            &run,
            Err(Error::Refused(Refusal::UnknownRun { run: 2, .. }))
        ),
        "{run:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_execution_prefix_selects_that_execution_and_its_log_directory() {
    let feature = delivered_with_one_recheck().await;
    let build = execution_of(&feature, "A", "build").await;

    let logs = feature
        .project
        .execution_logs(&build[..build.len() - 2])
        .await
        .expect("the execution is found");

    let [source] = logs.sources.as_slice() else {
        panic!("one source: {logs:?}");
    };
    assert_eq!((logs.subject.as_str(), logs.run), (SUBJECT, 1));
    assert_eq!(source.execution, build);
    assert_eq!(
        (source.subject.as_str(), source.run, source.item.as_deref()),
        (SUBJECT, 1, Some("A"))
    );
    assert_eq!(
        (source.step.as_str(), source.iteration, source.attempt),
        ("build", Some(1), 1)
    );
    assert_eq!(source.state, "succeeded");
    assert_eq!(
        source.directory,
        feature.root().join(".surt/logs").join(SUBJECT).join(&build)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn logs_say_whether_the_run_s_driver_is_live() {
    let feature = delivered_with_one_recheck().await;
    let build = execution_of(&feature, "A", "build").await;
    let live = async || {
        let run = feature.project.logs(SUBJECT, task("A")).await;
        let execution = feature.project.execution_logs(&build).await;
        (
            run.expect("the run").live,
            execution.expect("the execution").live,
        )
    };

    let driving = live().await;
    feature
        .dir
        .execute("UPDATE workflow_run SET driver_pid = NULL")
        .await;
    let gone = live().await;

    assert_eq!(driving, (true, true));
    assert_eq!(gone, (false, false));
}
