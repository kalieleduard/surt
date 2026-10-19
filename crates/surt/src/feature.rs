mod new_run;

use std::path::Path;

use clap::builder::NonEmptyStringValueParser;
use surt_engine::project::Project;

use crate::failure::Failure;
use crate::merge_flag::{MergeFlag, already_frozen};
use crate::phase::SPEC;

#[derive(clap::Args)]
pub(crate) struct FeatureArgs {
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    subject: String,
    #[arg(long, value_name = "ref", requires = "new_run")]
    base: Option<String>,
    #[arg(long = "new-run")]
    new_run: bool,
    #[arg(long, conflicts_with = "no_merge")]
    merge: bool,
    #[arg(long = "no-merge")]
    no_merge: bool,
}

pub(crate) async fn run(cwd: &Path, args: FeatureArgs) -> Result<(), Failure> {
    let project = Project::open(cwd).await?;
    let merge = MergeFlag::chosen(args.merge, args.no_merge);
    if args.new_run {
        return new_run::start(&project, &args.subject, args.base.as_deref(), merge).await;
    }
    let subject = project.resolve_subject(&args.subject).await?;
    if let Some(flag) = merge
        && let Some(status) = new_run::current(&project, &subject.name).await?
    {
        return Err(Failure::Refused(already_frozen(
            &subject.name,
            &status,
            flag,
        )));
    }
    SPEC.finish_cleanup(&project, &subject).await?;
    if subject.active_run.is_none() {
        return Err(new_run::suggested(&project, &subject.name).await);
    }
    SPEC.resume(&project, &subject).await
}
