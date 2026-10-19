use jiff::Zoned;
use surt_engine::project::{FreeName, Project, Slug};

use super::RunArgs;
use crate::failure::Failure;
use crate::message::Message;
use crate::retry::{Retry, quoted};

const NAME_PLACEHOLDER: &str = "--name <slug>";

pub(super) async fn free_name(
    project: &Project,
    args: &RunArgs,
    subject: Option<&str>,
) -> Result<FreeName, Failure> {
    let source = args.name.as_deref().or(subject).unwrap_or(&args.workflow);
    let slug = Slug::derive(source).ok_or_else(|| Failure::Refused(no_slug(args, subject)))?;
    if !args.new
        && let Some(active) = project.active_subject_with_slug(&slug).await?
    {
        return Err(Failure::Refused(active_slug(args, &active)));
    }
    Ok(project
        .free_subject_name(&slug, Zoned::now().date())
        .await?)
}

fn retry(args: &RunArgs) -> Retry {
    args.inputs.iter().fold(
        Retry::command(&["surt", "run", &args.workflow]),
        |retry, input| retry.word("--input").word(&input.quoted()),
    )
}

fn no_slug(args: &RunArgs, subject: Option<&str>) -> Message {
    let headline = match (&args.name, subject) {
        (Some(name), _) => format!("cannot derive a name from --name {}", quoted(name)),
        (None, Some(_)) => {
            "cannot derive a name from the subject input; pass --name <slug>".to_owned()
        }
        (None, None) => format!(
            "cannot derive a name from workflow {}; pass --name <slug>",
            quoted(&args.workflow)
        ),
    };
    let fix = retry(args).word(NAME_PLACEHOLDER);
    Message::new(headline).way("fix", fix.render())
}

fn active_slug(args: &RunArgs, active: &str) -> Message {
    let anew = retry(args)
        .option("--name", args.name.as_deref())
        .word("--new");
    Message::new(format!("{active} already has an active run"))
        .way("resume", format!("surt continue {active}"))
        .way("or", anew.render())
}
