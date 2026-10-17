use super::fixtures::{ProjectDir, default_config, draft_catalog, new_run};
use crate::project::{Error, Refusal};

#[tokio::test(flavor = "multi_thread")]
async fn a_second_active_root_run_for_the_same_subject_is_refused_and_writes_nothing() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    let first = project
        .create_run(new_run("draft"))
        .await
        .expect("the first run is created");

    let second = project.create_run(new_run("draft")).await;

    assert!(
        matches!(&second, Err(Error::Refused(Refusal::ActiveRunExists { run })) if *run == first),
        "{second:?}"
    );
    let counts: Vec<(i64, i64, i64, i64)> = dir
        .rows("SELECT (SELECT count(*) FROM subject), (SELECT count(*) FROM workflow_run), (SELECT count(*) FROM step_run), (SELECT count(*) FROM artifact)")
        .await;
    assert_eq!(counts, [(1, 1, 2, 2)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_subject_whose_run_finished_gets_a_new_root_run_with_the_next_ordinal() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    project
        .create_run(new_run("draft"))
        .await
        .expect("the first run is created");
    dir.execute("UPDATE workflow_run SET state = 'cancelled', finished_at = created_at")
        .await;

    let second = project
        .create_run(new_run("draft"))
        .await
        .expect("a new run is created");

    let runs: Vec<(String, i64, String)> = dir
        .rows("SELECT id, ordinal, state FROM workflow_run ORDER BY ordinal")
        .await;
    assert_eq!(runs.len(), 2);
    assert_eq!(
        (runs[1].0.as_str(), runs[1].1, runs[1].2.as_str()),
        (second.as_str(), 2, "active")
    );
    let subjects: Vec<(i64,)> = dir.rows("SELECT count(*) FROM subject").await;
    assert_eq!(subjects, [(1,)]);
}
