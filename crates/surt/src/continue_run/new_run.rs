use surt_engine::project::{
    Carry, Commit, Error, NewRun, Project, Refusal, RunSelector, RunStatus, StatusScope, Subject,
};

use super::{drive, finish_cleanup, new_run_command};
use crate::driving::local_time;
use crate::failure::Failure;
use crate::merge_flag::{MergeFlag, with_merge_line};
use crate::message::Message;
use crate::run::short_commit;
use crate::runtime;
use crate::status::state_names;

const FIELD_GAP: &str = "   ";

pub(super) async fn start(
    project: &Project,
    subject: Subject,
    inputs: Vec<(String, String)>,
    merge: Option<MergeFlag>,
) -> Result<(), Failure> {
    finish_cleanup(project, &subject).await?;
    let name = subject.name;
    let new_run = NewRun {
        workflow: latest_workflow(project, &name).await?,
        subject: name.clone(),
        inputs,
        carry: Carry::PreviousInputs,
        merge_into_base: merge.map(MergeFlag::mode),
        actor: project.actor().await?,
    };
    let runtime = runtime::select(project)?;
    let created = project.create_run(new_run).await?;
    let ordinal = current(project, &name)
        .await?
        .map_or(0, |status| status.run);
    with_merge_line(
        started(&name, ordinal, &created.git_refs),
        created.merge_into_base,
        merge,
    )
    .print_success();
    drive(project, &name, &created.run, &runtime).await
}

async fn latest_workflow(project: &Project, subject: &str) -> Result<String, Failure> {
    project.latest_workflow(subject).await?.ok_or_else(|| {
        Failure::from(Error::Refused(Refusal::UnknownSubject {
            reference: subject.to_owned(),
        }))
    })
}

pub(super) async fn suggested(project: &Project, subject: &str) -> Failure {
    match current(project, subject).await {
        Ok(latest) => Failure::Refused(no_active_run(subject, latest.as_ref())),
        Err(failure) => failure,
    }
}

pub(super) async fn current(
    project: &Project,
    subject: &str,
) -> Result<Option<RunStatus>, Failure> {
    let statuses = project
        .status(StatusScope::Subject(subject, RunSelector::Current))
        .await?;
    Ok(statuses.into_iter().next())
}

fn started(subject: &str, run: u32, git_refs: &[(String, Commit)]) -> Message {
    let mut headline = format!("run {run} of {subject}");
    for (input, commit) in git_refs {
        headline.push_str(&format!("{FIELD_GAP}{input}: {}", short_commit(commit)));
    }
    headline.push_str(&format!("{FIELD_GAP}branch surt/{subject}.{run}"));
    Message::new(headline)
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
        &format!("start run {} with the same inputs", latest.runs + 1),
        new_run_command(subject),
    )
}
