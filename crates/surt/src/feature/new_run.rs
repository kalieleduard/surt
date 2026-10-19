use surt_engine::project::{Commit, NewRun, Project, RunSelector, RunStatus, StatusScope};

use crate::driving::local_time;
use crate::failure::Failure;
use crate::merge_flag::{MergeFlag, with_merge_line};
use crate::message::Message;
use crate::phase::{SPEC, finish_cleanup};
use crate::run::short_commit;
use crate::runtime;
use crate::status::state_names;
use crate::template::{BASE_INPUT, FEATURE_WORKFLOW, REQUEST_INPUT};

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
    let mut new_run = next_run(&name, base, project.actor().await?);
    new_run.merge_into_base = merge.map(MergeFlag::mode);
    let runtime = runtime::select(project)?;
    let created = project.create_run(new_run).await?;
    let ordinal = current(project, &name)
        .await?
        .map_or(0, |status| status.run);
    let base = created
        .git_refs
        .iter()
        .find(|(input, _)| input == BASE_INPUT)
        .map(|(_, commit)| commit);
    with_merge_line(
        started(&name, ordinal, base),
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

fn next_run(subject: &str, base: Option<&str>, actor: String) -> NewRun {
    NewRun {
        subject: subject.to_owned(),
        workflow: FEATURE_WORKFLOW.to_owned(),
        inputs: base
            .map(|base| vec![(BASE_INPUT.to_owned(), base.to_owned())])
            .unwrap_or_default(),
        carried: vec![REQUEST_INPUT.to_owned()],
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

fn started(subject: &str, run: u32, base: Option<&Commit>) -> Message {
    let base = base
        .map(|commit| format!("   base {}", short_commit(commit)))
        .unwrap_or_default();
    Message::new(format!(
        "run {run} of {subject}{base}   branch surt/{subject}.{run}"
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
