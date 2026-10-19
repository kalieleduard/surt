use super::fixtures::{ProjectDir, SUBJECT, default_config, draft_catalog, new_run};

#[tokio::test(flavor = "multi_thread")]
async fn create_run_commits_the_subject_an_active_root_run_and_its_top_level_steps_pending() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;

    let run = project
        .create_run(new_run("draft"))
        .await
        .expect("the run is created")
        .run;

    let runs: Vec<(String, String, String, i64)> = dir
        .rows("SELECT w.id, s.name, w.state, w.ordinal FROM workflow_run w JOIN subject s ON s.id = w.subject_id WHERE w.id = w.root_run_id")
        .await;
    assert_eq!(
        runs,
        [(
            run.as_str().to_owned(),
            SUBJECT.to_owned(),
            "active".to_owned(),
            1
        )]
    );
    let steps: Vec<(String, String, String)> = dir
        .rows("SELECT step_key, kind, state FROM step_run ORDER BY step_key")
        .await;
    assert_eq!(
        steps,
        [
            (
                "drafting".to_owned(),
                "loop".to_owned(),
                "pending".to_owned()
            ),
            (
                "prepare".to_owned(),
                "operation".to_owned(),
                "pending".to_owned()
            ),
        ]
    );
    let events: Vec<(String, String)> = dir.rows("SELECT root_run_id, kind FROM event").await;
    assert_eq!(
        events,
        [(run.as_str().to_owned(), "run.created".to_owned())]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn run_created_carries_the_base_and_the_actor() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;

    project
        .create_run(new_run("draft"))
        .await
        .expect("the run is created");

    assert_eq!(
        dir.event_data("run.created").await,
        [serde_json::json!({ "base": { "sha": "abc" }, "actor": "dev@example.com" })]
    );
}
