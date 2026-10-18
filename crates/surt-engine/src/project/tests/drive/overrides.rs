use std::fs;

use super::super::fixtures::new_run;
use super::super::refusal;
use super::fixtures::{Feature, draft};
use crate::project::Refusal;
use crate::runtime::Script;

#[tokio::test(flavor = "multi_thread")]
async fn local_agent_overrides_refuse_creating_and_driving_runs_instead_of_being_ignored() {
    let feature = Feature::review().await;
    let agents = feature.dir.path().join(".surt/agents");
    fs::create_dir_all(agents.join("writer")).expect("an override directory");
    let mut second = new_run("review");
    second.subject = "20261017-other".to_owned();

    let created = feature.project.create_run(second).await;
    let driven = feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await;

    let expected = Refusal::AgentOverrides { path: agents };
    assert_eq!(refusal(created), expected);
    assert_eq!(refusal(driven.outcome), expected);
    let written: Vec<(i64, i64)> = feature
        .dir
        .rows("SELECT (SELECT count(*) FROM subject), (SELECT count(*) FROM execution)")
        .await;
    assert_eq!(written, [(1, 0)]);
}
