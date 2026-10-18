use std::path::Path;

use surt_engine::project::Project;

use crate::builtin;
use crate::failure::Failure;
use crate::phase::{SPEC, TASKS, active_run};

#[derive(clap::Args)]
pub(crate) struct PlanArgs {
    feature: String,
}

pub(crate) async fn run(cwd: &Path, args: PlanArgs) -> Result<(), Failure> {
    let project = Project::open(cwd, builtin::catalog()).await?;
    let subject = project.resolve_subject(&args.feature).await?;
    if TASKS.finish_cleanup(&project, &subject).await? {
        return Ok(());
    }
    SPEC.require_approved(&project, &subject.name, active_run(&subject)?)
        .await?;
    TASKS.resume(&project, &subject).await
}
