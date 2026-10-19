use std::path::PathBuf;

use super::super::fixtures::{SUBJECT, input};
use super::cleanup::{delivered_team, entries};
use super::fixtures::{ACTOR, Feature};
use super::integration::{integration_branch, task_branch_of};
use super::kept::{ITEMS, kept};
use super::planned::plan;
use crate::project::{Carry, DriveOutcome, Error, NewRun, Refusal};
use crate::runtime::Script;

const PLAN: &str = "plan.json";

fn next_run() -> NewRun {
    NewRun {
        subject: SUBJECT.to_owned(),
        workflow: "shipped".to_owned(),
        inputs: vec![input("base", "main")],
        carry: Carry::PreviousInputs,
        merge_into_base: None,
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
        .create_run(next_run())
        .await
        .expect("run 2 is created")
        .run;

    assert!(!feature.working_copy("").exists());
    feature.assert_untouched(&before);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_next_run_of_a_kept_subject_drives_on_its_own_dot_two_names() {
    let mut feature = kept().await;
    let before = feature.kept_run();
    feature.run = feature
        .project
        .create_run(next_run())
        .await
        .expect("run 2 is created")
        .run;

    feature
        .drive(Script::new().on("plan", 1, 1, plan(ITEMS)))
        .await
        .outcome
        .expect("the plan reaches its gate");
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the plan is approved");
    let driven = feature.drive_with(&delivered_team()).await;

    assert_eq!(
        driven.outcome.as_ref().expect("drive returns"),
        &DriveOutcome::Succeeded
    );
    let stem = format!("{SUBJECT}.2");
    for (key, _) in ITEMS {
        let worktree = feature.worktree(key);
        let second = worktree
            .parent()
            .and_then(|subject| subject.parent())
            .expect("the project's worktrees")
            .join(&stem)
            .join(key);
        assert!(second.is_dir(), "{}", second.display());
        let branch = format!("refs/heads/surt/item/{stem}/{key}");
        assert!(feature.sha(&branch).is_some(), "{branch}");
    }
    assert!(feature.sha(&format!("refs/heads/surt/{stem}")).is_some());
    feature.assert_untouched(&before);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_run_of_a_kept_subject_is_refused_while_its_working_copy_holds_an_unrecorded_edit() {
    let feature = kept().await;
    let before = feature.kept_run();
    feature.edit(PLAN, "[]\n");

    let created = feature.project.create_run(next_run()).await;

    assert_eq!(
        created.map(|_| ()).map_err(|error| match error {
            Error::Refused(refusal) => Some(refusal),
            Error::Internal(_) => None,
        }),
        Err(Some(Refusal::UnrecordedEditsBlockNewRun {
            subject: SUBJECT.to_owned(),
            run: 1,
            files: vec![PLAN.to_owned()],
        }))
    );
    let runs: Vec<(i64,)> = feature
        .dir
        .rows("SELECT count(*) FROM workflow_run WHERE parent_step_run_id IS NULL")
        .await;
    assert_eq!(runs, [(1,)]);
    assert!(feature.working_copy(PLAN).is_file());
    assert!(entries(&feature.working_copy("")).len() > 1);
    feature.assert_untouched(&before);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_run_of_a_kept_subject_refused_by_an_override_removes_nothing() {
    let feature = kept().await;
    let before = feature.kept_run();
    let files = entries(&feature.working_copy(""));
    feature.dir.write_agent_file(
        "planner",
        "agent.toml",
        "[inputs.style]\nformat = \"text\"\n",
    );

    let created = feature.project.create_run(next_run()).await;

    assert!(
        matches!(created, Err(Error::Refused(Refusal::InvalidDefinition(_)))),
        "{created:?}"
    );
    assert_eq!(entries(&feature.working_copy("")), files);
    feature.assert_untouched(&before);
}
