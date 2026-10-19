use std::fs;
use std::num::NonZeroU32;
use std::path::Path;

use super::super::fixtures::Catalog;
use super::super::fixtures::SUBJECT;
use super::fixtures::Feature;
use super::repository::commit_file;
use crate::project::{Config, DriveOutcome};
use crate::runtime::Script;

const PREPARE_WORKFLOW: &str = r#"
schema = 1
name = "prepared"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"
"#;

fn integration_branch() -> String {
    format!("refs/heads/surt/{SUBJECT}")
}

async fn prepared() -> Feature {
    Feature::on_repository(
        Catalog::new().workflow("prepared", PREPARE_WORKFLOW),
        "prepared",
        Config::default(),
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn prepare_creates_the_integration_branch_at_the_base_and_records_it() {
    let feature = prepared().await;
    let base = feature.base();

    let driven = feature.drive(Script::new()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(feature.sha(&integration_branch()), Some(base.clone()));
    let recorded: Vec<(String, String, String, Option<String>)> = feature
        .dir
        .rows("SELECT ref_name, start_sha, tip_sha, worktree_path FROM run_branch")
        .await;
    assert_eq!(recorded, [(integration_branch(), base.clone(), base, None)]);
    let executions: Vec<(String, Option<String>, String, Option<String>)> = feature
        .dir
        .rows("SELECT kind, operation_id, state, outcome FROM execution")
        .await;
    assert_eq!(
        executions,
        [(
            "operation".to_owned(),
            Some("git.create_run_branch".to_owned()),
            "succeeded".to_owned(),
            Some("done".to_owned())
        )]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_branch_already_at_the_base_is_the_one_prepare_meant_to_create() {
    let feature = prepared().await;
    let base = feature.base();
    feature.git(&["branch", &format!("surt/{SUBJECT}"), &base]);

    let driven = feature.drive(Script::new()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(feature.sha(&integration_branch()), Some(base));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_branch_found_elsewhere_blocks_prepare_and_is_never_adopted() {
    let feature = prepared().await;
    let base = feature.base();
    let elsewhere = commit_file(&feature.root(), "other.txt", "other\n");
    feature.git(&["branch", &format!("surt/{SUBJECT}"), &elsewhere]);

    let driven = feature.drive(Script::new()).await;

    assert!(
        matches!(
            driven.outcome.expect("drive returns"),
            DriveOutcome::Blocked { .. }
        ),
        "prepare blocks"
    );
    assert_eq!(feature.sha(&integration_branch()), Some(elsewhere.clone()));
    let failures: Vec<(String, String, String)> = feature
        .dir
        .rows("SELECT failure_class, error_code, error_message FROM execution")
        .await;
    assert_eq!(
        failures,
        [(
            "not_retryable".to_owned(),
            "integration_branch_moved".to_owned(),
            format!("surt/{SUBJECT} moved outside surt (expected {base}, found {elsewhere})")
        )]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_third_party_git_lock_is_waited_out_with_backoff_then_fails_the_attempt() {
    let config = Config {
        max_attempts: NonZeroU32::MIN,
        ..Config::default()
    };
    let feature = Feature::on_repository(
        Catalog::new().workflow("prepared", PREPARE_WORKFLOW),
        "prepared",
        config,
    )
    .await;
    let lock = feature
        .root()
        .join(".git/refs/heads/surt")
        .join(format!("{SUBJECT}.lock"));
    fs::create_dir_all(lock.parent().expect("a parent")).expect("the refs directory");
    fs::write(&lock, "").expect("a third-party lock");

    let driven = feature.drive(Script::new()).await;

    assert!(matches!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Blocked { .. }
    ));
    let failures: Vec<(String, String, String)> = feature
        .dir
        .rows("SELECT failure_class, error_code, error_message FROM execution")
        .await;
    let [(class, code, message)] = failures.as_slice() else {
        panic!("one failed execution: {failures:?}");
    };
    assert_eq!(
        (class.as_str(), code.as_str()),
        ("retryable", "git_lock_held")
    );
    assert_eq!(
        message.strip_prefix("git lock held: ").map(Path::new),
        Some(lock.as_path()),
        "{message}"
    );
    assert_eq!(feature.sha(&integration_branch()), None);
}
