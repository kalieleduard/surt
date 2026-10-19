use surt_engine::project::{Commit, NewRun, Project, RunSelector, RunStatus, StatusScope};

use super::{DEFAULT_BASE, MergeFlag, base_artifact, short_base, with_merge_line};
use crate::driving::local_time;
use crate::failure::Failure;
use crate::message::Message;
use crate::phase::{SPEC, finish_cleanup};
use crate::runtime;
use crate::status::state_names;
use crate::template::{FEATURE_WORKFLOW, REQUEST_ARTIFACT};

const COMMAND: &str = "feature";

pub(super) async fn start(
    project: &Project,
    reference: &str,
    base: Option<&str>,
    merge: Option<MergeFlag>,
) -> Result<(), Failure> {
    let subject = project.resolve_subject(reference).await?;
    finish_cleanup(project, &subject, COMMAND).await?;
    let name = subject.name;
    let commit = project
        .resolve_commit(base.unwrap_or(DEFAULT_BASE))
        .await?
        .ok_or_else(|| Failure::Refused(no_base(&name, base)))?;
    let mut new_run = next_run(&name, &commit, project.actor().await?);
    new_run.merge_into_base = merge.map(MergeFlag::mode);
    let runtime = runtime::select(project)?;
    let created = project.create_run(new_run).await?;
    let ordinal = current(project, &name)
        .await?
        .map_or(0, |status| status.run);
    with_merge_line(
        started(&name, ordinal, &commit),
        created.merge_into_base,
        merge,
    )
    .print_success();
    SPEC.drive(project, &name, &created.run, &runtime).await
}

pub(super) async fn suggested(project: &Project, subject: &str) -> Failure {
    match current(project, subject).await {
        Ok(latest) => Failure::Refused(no_active_run(subject, latest.as_ref())),
        Err(failure) => failure,
    }
}

fn next_run(subject: &str, base: &Commit, actor: String) -> NewRun {
    NewRun {
        subject: subject.to_owned(),
        workflow: FEATURE_WORKFLOW.to_owned(),
        initial_artifacts: vec![base_artifact(base)],
        carried: vec![REQUEST_ARTIFACT.to_owned()],
        merge_into_base: None,
        actor,
    }
}

pub(super) async fn current(
    project: &Project,
    subject: &str,
) -> Result<Option<RunStatus>, Failure> {
    let statuses = project
        .status(StatusScope::Feature(subject, RunSelector::Current))
        .await?;
    Ok(statuses.into_iter().next())
}

fn started(subject: &str, run: u32, base: &Commit) -> Message {
    Message::new(format!(
        "run {run} of {subject}   base {}   branch surt/{subject}.{run}",
        short_base(base)
    ))
}

fn no_active_run(subject: &str, latest: Option<&RunStatus>) -> Message {
    let Some(latest) = latest else {
        return Message::new(format!("{subject} has no active run"))
            .way("inspect", format!("surt status {subject}"));
    };
    let ended = latest
        .ended_at
        .and_then(local_time)
        .map(|time| format!(" at {time}"))
        .unwrap_or_default();
    Message::new(format!(
        "{subject} has no active run (run {} {}{ended})",
        latest.run,
        state_names(&latest.states).to_lowercase()
    ))
    .way(
        &format!("start run {} from the same request", latest.runs + 1),
        format!("surt feature {subject} --new-run"),
    )
}

fn no_base(subject: &str, base: Option<&str>) -> Message {
    let Some(base) = base else {
        return Message::new("HEAD does not name a commit; the repository has no commits yet")
            .way("fix", "git commit --allow-empty -m \"first commit\"");
    };
    Message::new(format!("--base {base} does not name a commit")).way(
        "fix",
        format!("surt feature {subject} --new-run --base <branch, tag or commit>"),
    )
}
