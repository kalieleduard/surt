use std::path::Path;

use super::evidence::{BUILD_WORKFLOW, COMMITTER, Committing, build_failures, change, committed};
use crate::project::{Config, DriveOutcome};
use crate::runtime::Script;

const CHECKS: &str = "checks = [{ kind = \"commit_evidence\" }]\n";

fn named_nothing(_: &Path) -> String {
    "0123456789abcdef0123456789abcdef01234567".to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn checks_dropped_from_the_agent_after_creation_are_still_enforced_on_the_run() {
    let feature = committed(BUILD_WORKFLOW, Config::default()).await;
    feature
        .dir
        .write_agent_file("builder", "agent.toml", &COMMITTER.replace(CHECKS, ""));
    let runtime = Committing::new(Script::new())
        .on(1, 1, named_nothing)
        .on(1, 2, change);

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let failures = build_failures(&feature).await;
    assert_eq!(
        failures,
        [(
            1,
            "invalid_delivery".to_owned(),
            "commit: 0123456789abcdef0123456789abcdef01234567 is not a commit".to_owned()
        )]
    );
    let frozen = feature.dir.frozen_agent("builder").await;
    assert_eq!(
        frozen["outputs"]["commit"]["checks"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
}
