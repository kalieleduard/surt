use std::fs;

use serde_json::{Value, json};

use super::super::fixtures::SUBJECT;
use super::fixtures::Feature;
use super::integration::{integration_branch, login};
use super::repository::commit_file;
use super::team::{INTEGRATED_TASK, Team, approved, builds, reviews, ships, team_catalog};
use crate::project::{
    BaseMerge, BaseMergeReport, Catalog, Commit, Config, DriveOutcome, GitConfig, MergeIntoBase,
    NotMerged,
};

const MERGED_WORKFLOW: &str = r#"
schema = 1
name = "merged"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.plan]
kind = "agent"
agent = "planner"
needs = ["prepare"]

[steps.approve]
kind = "approval"
artifact = "plan"
rejection_artifact = "plan-rejection"
needs = ["plan"]

[steps.implement]
kind = "per_item"
items = "plan"
workflow = "task"
pass = ["request"]
needs = ["approve"]

[steps.snapshot]
kind = "operation"
run = "git.snapshot_run_branch"
needs = ["implement"]

[steps.ship]
kind = "agent"
agent = "shipper"
needs = ["snapshot"]

[steps.merge]
kind = "operation"
run = "git.merge_into_base"
needs = ["ship"]
"#;

pub(super) fn merging(mode: MergeIntoBase) -> Config {
    Config {
        git: GitConfig {
            merge_into_base: mode,
        },
        ..Config::default()
    }
}

pub(super) fn merged_catalog() -> Catalog {
    team_catalog(INTEGRATED_TASK).workflow("merged", MERGED_WORKFLOW)
}

pub(super) async fn merged_feature(mode: MergeIntoBase, base_ref: Option<&str>) -> Feature {
    let feature = Feature::on_base(merged_catalog(), "merged", merging(mode), base_ref).await;
    approved(feature, &[("A", &[])]).await
}

pub(super) fn delivering_team() -> Team {
    Team::new()
        .on("A/build", 1, 1, builds(login))
        .on("A/review", 1, 1, reviews("approved"))
        .on("ship", 1, 1, ships())
}

impl Feature {
    pub(super) async fn merge_done(&self) -> Vec<Value> {
        let rows: Vec<(String,)> = self
            .dir
            .rows(
                "SELECT e.data FROM event e JOIN step_run s ON s.id = e.step_run_id \
                 WHERE s.step_key = 'merge' AND e.kind = 'step.done' ORDER BY e.seq",
            )
            .await;
        rows.into_iter()
            .map(|(data,)| serde_json::from_str(&data).expect("event data is JSON"))
            .collect()
    }

    pub(super) fn leave_main(&self) {
        self.git(&["checkout", "--quiet", "--detach"]);
    }

    pub(super) fn hook(&self, name: &str, script: &str) {
        let path = self.root().join(".git").join("hooks").join(name);
        fs::write(&path, script).expect("the hook is written");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                .expect("the hook is executable");
        }
    }

    pub(super) fn unhook(&self, name: &str) {
        let path = self.root().join(".git").join("hooks").join(name);
        fs::remove_file(path).expect("the hook is removed");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_base_branch_checked_out_nowhere_is_fast_forwarded_to_the_delivered_tip() {
    let feature = merged_feature(MergeIntoBase::FfOnly, Some("main")).await;
    feature.leave_main();
    let base = feature.base();

    let driven = feature.drive_with(&delivering_team()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let tip = feature
        .sha(&integration_branch())
        .expect("the integration branch");
    assert_ne!(tip, base);
    assert_eq!(feature.sha("main"), Some(tip.clone()));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": base, "to": tip })]
    );
    assert_eq!(
        feature.git(&["reflog", "-1", "--format=%gs", "main"]),
        format!("surt: fast-forward main to surt/{SUBJECT}")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_project_reports_the_recorded_merge_with_the_base_it_started_from() {
    let feature = merged_feature(MergeIntoBase::FfOnly, Some("main")).await;
    let moved = commit_file(&feature.root(), "NEWS.md", "news\n");
    feature.leave_main();
    let base = feature.git(&["rev-parse", "main~1"]);
    assert_eq!(
        feature.project.base_merge(&feature.run).await.ok(),
        Some(None)
    );

    feature.drive_with(&delivering_team()).await;

    assert_eq!(
        feature.project.base_merge(&feature.run).await.ok(),
        Some(Some(BaseMergeReport {
            base: Commit {
                sha: base,
                branch: Some("main".to_owned()),
            },
            merge: BaseMerge::Skipped(NotMerged::BaseMoved { found: Some(moved) }),
        }))
    );
}
