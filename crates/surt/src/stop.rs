use std::path::Path;

use surt_engine::project::{Project, RunSelector, StatusScope, StopRequest};

use crate::builtin;
use crate::failure::Failure;
use crate::message::{Mark, Message};
use crate::status::resume_command;
use crate::symbols::ELLIPSIS;

#[derive(clap::Args)]
pub(crate) struct StopArgs {
    feature: String,
}

pub(crate) async fn run(cwd: &Path, args: StopArgs) -> Result<(), Failure> {
    let project = Project::open(cwd, builtin::catalog()).await?;
    let subject = project.resolve_subject(&args.feature).await?;
    let request = project.request_stop(&subject.name).await?;
    requested(&request).print(Mark::Waiting);
    project.wait_for_driver(&request.run).await?;
    let scope = StatusScope::Feature(&subject.name, RunSelector::Current);
    for status in project.status(scope).await? {
        Message::new(format!("stopped. resumable: {}", resume_command(&status))).print_success();
    }
    Ok(())
}

fn requested(request: &StopRequest) -> Message {
    let executions = match request.executions {
        1 => "1 execution".to_owned(),
        count => format!("{count} executions"),
    };
    Message::new(format!(
        "stop requested; driver pid {} is stopping {executions}{ELLIPSIS}",
        request.driver
    ))
}
