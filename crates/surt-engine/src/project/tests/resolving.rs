use super::fixtures::{ProjectDir, default_config, draft_catalog, new_run};
use crate::project::{Candidate, Error, Project, Refusal, RunId, RunState, Subject};

async fn project_with(dir: &ProjectDir, subjects: &[&str]) -> (Project, Vec<RunId>) {
    let project = dir.project(draft_catalog(), default_config()).await;
    let mut runs = Vec::new();
    for subject in subjects {
        let mut run = new_run("draft");
        run.subject = (*subject).to_owned();
        runs.push(
            project
                .create_run(run)
                .await
                .expect("the run is created")
                .run,
        );
    }
    (project, runs)
}

fn interrupted(name: &str) -> Candidate {
    Candidate {
        name: name.to_owned(),
        states: vec![RunState::Interrupted],
        awaiting: None,
    }
}

fn refused(result: Result<Subject, Error>) -> Refusal {
    match result {
        Err(Error::Refused(refusal)) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_full_name_a_unique_slug_or_a_unique_prefix_names_the_feature() {
    let dir = ProjectDir::new();
    let (project, runs) = project_with(&dir, &["20261017-add-login", "20261016-fix-cache"]).await;
    let expected = Subject {
        name: "20261017-add-login".to_owned(),
        active_run: Some(runs[0].clone()),
        pending_cleanup: None,
    };

    for reference in ["20261017-add-login", "add-login", "20261017-a"] {
        let resolved = project.resolve_subject(reference).await;

        assert_eq!(resolved.expect(reference), expected, "{reference}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_full_name_wins_over_the_longer_names_it_prefixes() {
    let dir = ProjectDir::new();
    let (project, _) = project_with(&dir, &["20261017-add-login", "20261017-add-login-2"]).await;

    let resolved = project
        .resolve_subject("20261017-add-login")
        .await
        .expect("the full name");

    assert_eq!(resolved.name, "20261017-add-login");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_ambiguous_slug_or_prefix_refuses_listing_the_candidates() {
    let dir = ProjectDir::new();
    let (project, _) = project_with(
        &dir,
        &[
            "20261017-add-login",
            "20261015-add-login",
            "20261017-add-logout",
        ],
    )
    .await;

    let by_slug = refused(project.resolve_subject("add-login").await);
    let by_prefix = refused(project.resolve_subject("20261017-add-log").await);

    assert_eq!(
        by_slug,
        Refusal::AmbiguousSubject {
            reference: "add-login".to_owned(),
            candidates: vec![
                interrupted("20261015-add-login"),
                interrupted("20261017-add-login")
            ],
        }
    );
    assert_eq!(
        by_prefix,
        Refusal::AmbiguousSubject {
            reference: "20261017-add-log".to_owned(),
            candidates: vec![
                interrupted("20261017-add-login"),
                interrupted("20261017-add-logout")
            ],
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_reference_that_matches_nothing_is_refused() {
    let dir = ProjectDir::new();
    let (project, _) = project_with(&dir, &["20261017-add-login"]).await;

    let unknown = refused(project.resolve_subject("login").await);

    assert_eq!(
        unknown,
        Refusal::UnknownSubject {
            reference: "login".to_owned()
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_feature_without_an_active_run_is_still_resolved() {
    let dir = ProjectDir::new();
    let (project, _) = project_with(&dir, &["20261017-add-login"]).await;
    dir.execute("UPDATE workflow_run SET state = 'cancelled', finished_at = created_at")
        .await;

    let resolved = project
        .resolve_subject("add-login")
        .await
        .expect("the slug");

    assert_eq!(resolved.active_run, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn only_an_exact_full_name_is_a_subject_name() {
    let dir = ProjectDir::new();
    let (project, _) = project_with(&dir, &["20261017-add-login"]).await;

    let exact = project
        .subject_named("20261017-add-login")
        .await
        .expect("the query runs");
    let slug = project
        .subject_named("add-login")
        .await
        .expect("the query runs");

    assert_eq!(
        exact.map(|subject| subject.name).as_deref(),
        Some("20261017-add-login")
    );
    assert_eq!(slug, None);
}
