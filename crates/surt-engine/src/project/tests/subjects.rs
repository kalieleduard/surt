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
async fn a_new_subject_takes_the_local_date_and_the_slug() {
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
async fn a_cancelled_subject_keeps_its_name_taken() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    create(&project, "20261017-add-login").await;
    dir.execute("UPDATE workflow_run SET state = 'cancelled', finished_at = created_at")
        .await;

    let name = project
        .free_subject_name(&add_login(), date(2026, 10, 17))
        .await
        .expect("a name");

    assert_eq!(name.name, "20261017-add-login-2");
}

async fn next_numbered(project: &Project) -> FreeName {
    project
        .next_numbered_subject_name(date(2026, 10, 17))
        .await
        .expect("a name")
}

#[tokio::test(flavor = "multi_thread")]
async fn the_first_numbered_subject_of_the_day_is_one() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;

    let name = next_numbered(&project).await;

    assert_eq!(name.name, "20261017-1");
    assert!(!name.is_suffixed());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_next_number_is_one_more_than_the_highest_of_the_day() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    create(&project, "20261017-1").await;
    create(&project, "20261017-2").await;

    assert_eq!(next_numbered(&project).await.name, "20261017-3");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_gap_in_the_numbers_is_not_filled() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    create(&project, "20261017-1").await;
    create(&project, "20261017-5").await;

    assert_eq!(next_numbered(&project).await.name, "20261017-6");
}

#[tokio::test(flavor = "multi_thread")]
async fn numbers_are_compared_numerically() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    create(&project, "20261017-9").await;
    create(&project, "20261017-10").await;

    assert_eq!(next_numbered(&project).await.name, "20261017-11");
}

#[tokio::test(flavor = "multi_thread")]
async fn named_subjects_do_not_count_towards_the_number() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    create(&project, "20261017-add-login").await;
    create(&project, "20261017-add-login-2").await;

    assert_eq!(next_numbered(&project).await.name, "20261017-1");
}

#[tokio::test(flavor = "multi_thread")]
async fn subjects_of_other_dates_do_not_count_towards_the_number() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    create(&project, "20261016-4").await;

    assert_eq!(next_numbered(&project).await.name, "20261017-1");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cancelled_numbered_subject_keeps_its_number_taken() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    create(&project, "20261017-1").await;
    dir.execute("UPDATE workflow_run SET state = 'cancelled', finished_at = created_at")
        .await;

    assert_eq!(next_numbered(&project).await.name, "20261017-2");
}
