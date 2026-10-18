use std::path::PathBuf;

use super::super::fixtures::{SUBJECT, initial};
use super::cleanup::entries;
use super::fixtures::{ACTOR, Feature};
use super::integration::{integration_branch, task_branch_of};
use super::kept::{ITEMS, kept};
use crate::project::NewRun;

const REQUEST: &str = "request";

fn next_run(feature: &Feature) -> NewRun {
    let base_json = format!(r#"{{"ref": "main", "sha": "{}"}}"#, feature.base());
    NewRun {
        subject: SUBJECT.to_owned(),
        workflow: "shipped".to_owned(),
        initial_artifacts: vec![initial("base", "base.json", base_json.as_bytes())],
        carried: vec![REQUEST.to_owned()],
        actor: ACTOR.to_owned(),
    }
}

struct KeptRun {
    worktrees: Vec<PathBuf>,
    refs: Vec<(String, Option<String>)>,
    salvage: String,
    executions: Vec<String>,
}

impl Feature {
    fn kept_run(&self) -> KeptRun {
        let keys = ITEMS.iter().map(|(key, _)| *key);
        let mut refs: Vec<String> = keys.clone().map(task_branch_of).collect();
        refs.push(integration_branch());
        KeptRun {
            worktrees: keys.map(|key| self.worktree(key)).collect(),
            refs: refs
                .into_iter()
                .map(|name| {
                    let sha = self.sha(&name);
                    (name, sha)
                })
                .collect(),
            salvage: self.surt_refs(),
            executions: entries(&self.executions_dir()),
        }
    }

    fn assert_untouched(&self, before: &KeptRun) {
        for worktree in &before.worktrees {
            assert!(worktree.is_dir(), "{}", worktree.display());
        }
        for (name, sha) in &before.refs {
            assert_eq!(&self.sha(name), sha, "{name}");
        }
        assert!(self.surt_refs().contains(&before.salvage));
        let executions = entries(&self.executions_dir());
        for execution in &before.executions {
            assert!(executions.contains(execution), "{execution}");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_run_of_a_kept_subject_clears_the_baseline_working_copy_and_keeps_the_kept_run() {
    let mut feature = kept().await;
    let before = feature.kept_run();
    assert!(!entries(&feature.working_copy("")).is_empty());

    feature.run = feature
        .project
        .create_run(next_run(&feature))
        .await
        .expect("run 2 is created");

    assert!(!feature.working_copy("").exists());
    feature.assert_untouched(&before);
}
