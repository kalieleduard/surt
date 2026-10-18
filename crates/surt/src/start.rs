use std::path::Path;

use surt_engine::project::Project;

use crate::builtin;
use crate::driving::Driven;
use crate::failure::Failure;
use crate::phase::{TASKS, active_run};
use crate::runtime;

const COMMAND: &str = "start";

#[derive(clap::Args)]
pub(crate) struct StartArgs {
    feature: String,
}

pub(crate) async fn run(cwd: &Path, args: StartArgs) -> Result<(), Failure> {
    let project = Project::open(cwd, builtin::catalog()).await?;
    let subject = project.resolve_subject(&args.feature).await?;
    let run = active_run(&subject)?;
    TASKS.require_approved(&project, &subject.name, run).await?;
    let runtime = runtime::select()?;
    let driven = Driven {
        project: &project,
        subject: &subject.name,
        run,
        command: COMMAND,
    };
    driven.drive(&runtime).await
}
