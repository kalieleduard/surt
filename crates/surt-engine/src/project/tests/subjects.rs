use jiff::civil::date;

use super::fixtures::{ProjectDir, default_config, draft_catalog, new_run};
use crate::project::{FreeName, Project, Slug};

fn add_login() -> Slug {
    Slug::derive("add login").expect("a slug")
}

async fn create(project: &Project, subject: &str) {
    let mut run = new_run("draft");
    run.subject = subject.to_owned();
    project.create_run(run).await.expect("the run is created");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_subject_is_named_after_the_local_date_and_the_slug() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;

    let name = project
        .free_subject_name(&add_login(), date(2026, 1, 5))
        .await
        .expect("a name");

    assert_eq!(name.name, "20260105-add-login");
    assert!(!name.is_suffixed());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_taken_name_gets_the_next_free_numeric_suffix() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    create(&project, "20261017-add-login").await;
    create(&project, "20261017-add-login-2").await;
    create(&project, "20261017-add-login-page").await;

    let same_day = project
        .free_subject_name(&add_login(), date(2026, 10, 17))
        .await
        .expect("a name");
    let next_day = project
        .free_subject_name(&add_login(), date(2026, 10, 18))
        .await
        .expect("a name");

    assert_eq!(
        same_day,
        FreeName {
            name: "20261017-add-login-3".to_owned(),
            stem: "20261017-add-login".to_owned(),
        }
    );
    assert!(same_day.is_suffixed());
    assert_eq!(next_day.name, "20261018-add-login");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_active_run_of_the_same_slug_is_found_across_dates_and_suffixes() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    create(&project, "20261016-add-login-2").await;
    create(&project, "20261016-add-login-page").await;

    let active = project
        .active_subject_with_slug(&add_login())
        .await
        .expect("the query runs");

    assert_eq!(active.as_deref(), Some("20261016-add-login-2"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cancelled_run_does_not_hold_its_slug_and_its_name_stays_taken() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    create(&project, "20261017-add-login").await;
    dir.execute("UPDATE workflow_run SET state = 'cancelled', finished_at = created_at")
        .await;

    let active = project
        .active_subject_with_slug(&add_login())
        .await
        .expect("the query runs");
    let name = project
        .free_subject_name(&add_login(), date(2026, 10, 17))
        .await
        .expect("a name");

    assert_eq!(active, None);
    assert_eq!(name.name, "20261017-add-login-2");
}
