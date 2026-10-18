use super::fixtures::{ProjectDir, SUBJECT, default_config, draft_catalog, initial, new_run};
use crate::project::{
    Error, KnownRun, Project, Refusal, RunSelector, RunState, ShowRequest, StatusScope,
};

const CANCEL_ALL: &str = "UPDATE workflow_run SET state = 'cancelled', finished_at = created_at";

async fn two_runs(dir: &ProjectDir) -> Project {
    let project = dir.project(draft_catalog(), default_config()).await;
    project
        .create_run(new_run("draft"))
        .await
        .expect("run 1 is created");
    dir.execute(CANCEL_ALL).await;
    let mut second = new_run("draft");
    second.initial_artifacts[0] = initial("request", "request.txt", b"add login again");
    project.create_run(second).await.expect("run 2 is created");
    project
}

async fn run_states(project: &Project, run: RunSelector) -> Vec<(u32, Vec<RunState>)> {
    project
        .status(StatusScope::Feature(SUBJECT, run))
        .await
        .expect("the status reads")
        .into_iter()
        .map(|status| (status.run, status.states))
        .collect()
}

fn request(run: RunSelector) -> ShowRequest<'static> {
    ShowRequest {
        artifact: "request",
        revision: None,
        item: None,
        iteration: None,
        file: None,
        run,
    }
}

fn refused<T: std::fmt::Debug>(result: Result<T, Error>) -> Refusal {
    match result {
        Err(Error::Refused(refusal)) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn reads_default_to_the_active_run_and_an_ordinal_reads_an_earlier_one() {
    let dir = ProjectDir::new();
    let project = two_runs(&dir).await;

    let current = run_states(&project, RunSelector::Current).await;
    let first = run_states(&project, RunSelector::Ordinal(1)).await;
    let shown_now = project.show(SUBJECT, request(RunSelector::Current)).await;
    let shown_then = project
        .show(SUBJECT, request(RunSelector::Ordinal(1)))
        .await;

    assert_eq!(current, [(2, vec![RunState::Interrupted])]);
    assert_eq!(first, [(1, vec![RunState::Cancelled])]);
    assert_eq!(shown_now.expect("run 2").content, b"add login again");
    assert_eq!(shown_then.expect("run 1").content, b"add login");
}

#[tokio::test(flavor = "multi_thread")]
async fn without_an_active_run_reads_default_to_the_latest_run() {
    let dir = ProjectDir::new();
    let project = two_runs(&dir).await;
    dir.execute(CANCEL_ALL).await;

    let current = run_states(&project, RunSelector::Current).await;
    let artifacts = project
        .artifacts(SUBJECT, RunSelector::Ordinal(1))
        .await
        .expect("run 1 has artifacts");

    assert_eq!(current, [(2, vec![RunState::Cancelled])]);
    let mut names: Vec<_> = artifacts
        .into_iter()
        .map(|artifact| artifact.name)
        .collect();
    names.sort();
    assert_eq!(names, ["base", "request"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_run_is_refused_listing_the_runs_with_their_states() {
    let dir = ProjectDir::new();
    let project = two_runs(&dir).await;

    let by_status = refused(
        project
            .status(StatusScope::Feature(SUBJECT, RunSelector::Ordinal(3)))
            .await,
    );
    let by_show = refused(
        project
            .show(SUBJECT, request(RunSelector::Ordinal(3)))
            .await,
    );
    let by_artifacts = refused(project.artifacts(SUBJECT, RunSelector::Ordinal(0)).await);

    let runs = vec![
        KnownRun {
            run: 1,
            states: vec![RunState::Cancelled],
        },
        KnownRun {
            run: 2,
            states: vec![RunState::Interrupted],
        },
    ];
    let unknown = |run| Refusal::UnknownRun {
        subject: SUBJECT.to_owned(),
        run,
        runs: runs.clone(),
    };
    assert_eq!(by_status, unknown(3));
    assert_eq!(by_show, unknown(3));
    assert_eq!(by_artifacts, unknown(0));
}
