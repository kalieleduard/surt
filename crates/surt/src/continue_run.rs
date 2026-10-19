mod new_run;

use std::path::Path;

use clap::builder::NonEmptyStringValueParser;
use surt_engine::project::{Project, RunId, Subject};
use surt_engine::runtime::AgentRuntime;

use crate::driving::Driven;
use crate::failure::Failure;
use crate::merge_flag::{MergeFlag, already_frozen};
use crate::message::Message;
use crate::run::inputs::{self, InputArg};
use crate::runtime;

#[derive(clap::Args)]
pub(crate) struct ContinueArgs {
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    subject: String,
    #[arg(long = "new-run")]
    new_run: bool,
    #[arg(
        long = "input",
        value_name = "name=value",
        value_parser = inputs::parse,
        requires = "new_run"
    )]
    inputs: Vec<InputArg>,
    #[arg(long, conflicts_with = "no_merge")]
    merge: bool,
    #[arg(long = "no-merge")]
    no_merge: bool,
}

pub(crate) fn continue_command(subject: &str) -> String {
    format!("surt continue {subject}")
}

pub(crate) fn new_run_command(subject: &str) -> String {
    format!("surt continue {subject} --new-run")
}

pub(crate) async fn run(cwd: &Path, args: ContinueArgs) -> Result<(), Failure> {
    let project = Project::open(cwd).await?;
    let merge = MergeFlag::chosen(args.merge, args.no_merge);
    let subject = project.resolve_subject(&args.subject).await?;
    if args.new_run {
        let inputs = inputs::read(cwd, &args.inputs)?;
        return new_run::start(&project, subject, inputs, merge).await;
    }
    if let Some(flag) = merge
        && let Some(status) = new_run::current(&project, &subject.name).await?
    {
        return Err(Failure::Refused(already_frozen(
            &subject.name,
            &status,
            flag,
        )));
    }
    if finish_cleanup(&project, &subject).await? {
        return Ok(());
    }
    let Some(run) = &subject.active_run else {
        return Err(new_run::suggested(&project, &subject.name).await);
    };
    let runtime = runtime::select(&project)?;
    drive(&project, &subject.name, run, &runtime).await
}

pub(crate) async fn finish_cleanup(project: &Project, subject: &Subject) -> Result<bool, Failure> {
    let (None, Some(run)) = (&subject.active_run, &subject.pending_cleanup) else {
        return Ok(false);
    };
    let runtime = runtime::select(project)?;
    drive(project, &subject.name, run, &runtime).await?;
    Ok(true)
}

async fn drive(
    project: &Project,
    subject: &str,
    run: &RunId,
    runtime: &impl AgentRuntime,
) -> Result<(), Failure> {
    let driven = Driven {
        project,
        subject,
        run,
    };
    driven.drive(runtime).await
}

pub(crate) fn active_run(subject: &Subject) -> Result<&RunId, Failure> {
    subject.active_run.as_ref().ok_or_else(|| {
        Failure::Refused(
            Message::new(format!("{} has no active run", subject.name))
                .way("inspect", format!("surt status {}", subject.name)),
        )
    })
}
