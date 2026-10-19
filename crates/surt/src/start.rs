use std::path::Path;

use surt_engine::project::Project;

use crate::driving::Driven;
use crate::failure::Failure;
use crate::phase::{TASKS, active_run, finish_cleanup};
use crate::runtime;

const COMMAND: &str = "start";

#[derive(clap::Args)]
pub(crate) struct StartArgs {
    feature: String,
}

pub(crate) async fn run(cwd: &Path, args: StartArgs) -> Result<(), Failure> {
    let project = Project::open(cwd).await?;
    let subject = project.resolve_subject(&args.feature).await?;
    if finish_cleanup(&project, &subject, COMMAND).await? {
        return Ok(());
    }
    let run = active_run(&subject)?;
    TASKS.require_approved(&project, &subject.name, run).await?;
    let runtime = runtime::select(&project)?;
    let driven = Driven {
        project: &project,
        subject: &subject.name,
        run,
        command: COMMAND,
    };
    driven.drive(&runtime).await
}
