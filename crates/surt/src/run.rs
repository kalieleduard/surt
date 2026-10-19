mod created;
pub(crate) mod inputs;
mod naming;

use std::path::Path;

use clap::builder::NonEmptyStringValueParser;
use surt_engine::project::{Carry, NewRun, Project};

use crate::driving::Driven;
use crate::failure::Failure;
use crate::merge_flag::{MergeFlag, with_merge_line};
use crate::runtime;
pub(crate) use created::short_commit;
use inputs::InputArg;

#[derive(clap::Args)]
pub(crate) struct RunArgs {
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    workflow: String,
    #[arg(long = "input", value_name = "name=value", value_parser = inputs::parse)]
    inputs: Vec<InputArg>,
    #[arg(long, value_name = "slug")]
    name: Option<String>,
    #[arg(long)]
    new: bool,
    #[arg(long, conflicts_with = "no_merge")]
    merge: bool,
    #[arg(long = "no-merge")]
    no_merge: bool,
}

pub(crate) async fn run(cwd: &Path, args: RunArgs) -> Result<(), Failure> {
    let project = Project::open(cwd).await?;
    let merge = MergeFlag::chosen(args.merge, args.no_merge);
    let inputs = inputs::read(cwd, &args.inputs)?;
    let subject = project.subject_input(&args.workflow, &inputs)?;
    let name = naming::free_name(&project, &args, subject).await?;
    let new_run = NewRun {
        subject: name.name.clone(),
        workflow: args.workflow.clone(),
        inputs,
        carry: Carry::Nothing,
        merge_into_base: merge.map(MergeFlag::mode),
        actor: project.actor().await?,
    };
    let runtime = runtime::select(&project)?;
    let created_run = project.create_run(new_run).await?;
    let line = created::line(&name, &args.workflow, &created_run.git_refs);
    with_merge_line(line, created_run.merge_into_base, merge).print_success();
    let driven = Driven {
        project: &project,
        subject: &name.name,
        run: &created_run.run,
    };
    driven.drive(&runtime).await
}
