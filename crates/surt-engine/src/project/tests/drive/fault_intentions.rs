use super::faults::assert_crashed;
use super::fixtures::{ACTOR, Feature};
use super::integration::{integration_branch, login};
use super::planned::plan;
use super::team::{INTEGRATED_TASK, Team, builds, built, reviews, team_catalog};
use crate::project::{Checkpoint, Config, DriveOutcome, Project};
use crate::runtime::{
    AgentRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink, RuntimeRequest, Script,
    StopSignal,
};

struct CrashingAfter<'a> {
    team: Team,
    step: &'static str,
    checkpoint: Checkpoint,
    project: &'a Project,
}

impl AgentRuntime for CrashingAfter<'_> {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        if request.step.step == self.step {
            self.project.crash_after(self.checkpoint);
        }
        self.team.execute(request, events, stop).await
    }
}

fn task_a() -> Team {
    Team::new()
        .on("A/build", 1, 1, builds(login))
        .on("A/review", 1, 1, reviews("approved"))
}

impl Feature {
    fn merges(&self) -> String {
        self.git(&["rev-list", "--merges", "--count", &integration_branch()])
    }

    async fn step_attempts(&self, step: &str) -> Vec<(i64, String)> {
        let sql = format!(
            "SELECT e.attempt, e.state FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             WHERE s.step_key = '{step}' ORDER BY e.attempt"
        );
        self.dir.rows(&sql).await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_right_after_a_branch_is_intended_creates_it_once_on_the_next_drive() {
    let catalog = team_catalog(INTEGRATED_TASK);
    let feature = Feature::on_repository(catalog, "built", Config::default()).await;
    let planned = Script::new().on("plan", 1, 1, plan(&[("A", &[])]));
    feature.project.crash_after(Checkpoint::Intention);

    assert_crashed(
        feature.drive(planned.clone()).await.outcome,
        Checkpoint::Intention,
    );
    assert_eq!(feature.sha(&integration_branch()), None);

    let resumed = feature.drive(planned).await.outcome;
    assert!(
        matches!(resumed, Ok(DriveOutcome::AtGate { .. })),
        "{resumed:?}"
    );
    assert_eq!(feature.sha(&integration_branch()), Some(feature.base()));
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the plan is approved");
    let finished = feature.drive_with(&task_a()).await.outcome;
    assert_eq!(finished.expect("drive returns"), DriveOutcome::Succeeded);
    assert_eq!(
        feature.step_attempts("prepare").await,
        [(1, "interrupted".to_owned()), (2, "succeeded".to_owned())]
    );
    let branches: Vec<(String,)> = feature
        .dir
        .rows("SELECT ref_name FROM run_branch WHERE worktree_path IS NULL")
        .await;
    assert_eq!(branches, [(integration_branch(),)]);
    assert_eq!(feature.merges(), "1");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_right_after_a_merge_is_intended_merges_the_task_exactly_once() {
    let feature = built(INTEGRATED_TASK, &[("A", &[])], Config::default()).await;
    let crashing = CrashingAfter {
        team: task_a(),
        step: "A/review",
        checkpoint: Checkpoint::Intention,
        project: &feature.project,
    };

    assert_crashed(
        feature.drive_with(&crashing).await.outcome,
        Checkpoint::Intention,
    );
    assert_eq!(feature.sha(&integration_branch()), Some(feature.base()));
    let intended: Vec<(String,)> = feature.dir.rows("SELECT ref_name FROM ref_move").await;
    assert_eq!(intended, [(integration_branch(),)]);

    let resumed = feature.drive_with(&Team::new()).await.outcome;
    assert_eq!(resumed.expect("drive returns"), DriveOutcome::Succeeded);
    assert_eq!(feature.merges(), "1");
    assert_eq!(
        feature.integrations().await,
        [("A".to_owned(), 1, "merged".to_owned())]
    );
    let left: Vec<(String,)> = feature.dir.rows("SELECT ref_name FROM ref_move").await;
    assert!(left.is_empty(), "{left:?}");
    let reviews: Vec<(i64, String)> = feature.step_attempts("review").await;
    assert_eq!(reviews, [(1, "succeeded".to_owned())]);
}
