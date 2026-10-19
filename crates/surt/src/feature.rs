mod merge_flag;
mod new_run;
mod retry;

use std::path::Path;

use clap::builder::NonEmptyStringValueParser;
use jiff::Zoned;
use surt_engine::project::{
    ArtifactFile, Commit, FreeName, InitialArtifact, LocalFiles, NewRun, Project, Slug,
};

use crate::builtin::{self, BASE_ARTIFACT, FEATURE_WORKFLOW, REQUEST_ARTIFACT};
use crate::failure::Failure;
use crate::message::Message;
use crate::phase::SPEC;
use crate::runtime;
use merge_flag::{MergeFlag, already_frozen, with_merge_line};
use retry::Retry;
pub(crate) use retry::quoted;

const DEFAULT_BASE: &str = "HEAD";
const REQUEST_FILE: &str = "request.txt";
const BASE_FILE: &str = "base.json";
const SHORT_SHA_CHARS: usize = 7;

#[derive(clap::Args)]
pub(crate) struct FeatureArgs {
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    description: String,
    #[arg(long, value_name = "slug")]
    name: Option<String>,
    #[arg(long, value_name = "ref")]
    base: Option<String>,
    #[arg(long)]
    new: bool,
    #[arg(long = "new-run", conflicts_with_all = ["name", "new"])]
    new_run: bool,
    #[arg(long, conflicts_with = "no_merge")]
    merge: bool,
    #[arg(long = "no-merge")]
    no_merge: bool,
}

pub(crate) async fn run(cwd: &Path, args: FeatureArgs) -> Result<(), Failure> {
    let project = Project::open(cwd, builtin::catalog()).await?;
    let merge = MergeFlag::chosen(args.merge, args.no_merge);
    if args.new_run {
        return new_run::start(&project, &args.description, args.base.as_deref(), merge).await;
    }
    if let Some(subject) = project.subject_named(&args.description).await? {
        if let Some(flag) = merge
            && subject.active_run.is_some()
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
        return SPEC.resume(&project, &subject).await;
    }
    create(&project, &args, merge).await
}

async fn create(
    project: &Project,
    args: &FeatureArgs,
    merge: Option<MergeFlag>,
) -> Result<(), Failure> {
    let source = args.name.as_deref().unwrap_or(&args.description);
    let slug = Slug::derive(source).ok_or_else(|| Failure::Refused(no_slug(args)))?;
    if !args.new
        && let Some(active) = project.active_subject_with_slug(&slug).await?
    {
        return Err(Failure::Refused(active_slug(args, &active)));
    }
    let base_ref = args.base.as_deref().unwrap_or(DEFAULT_BASE);
    let base = project
        .resolve_commit(base_ref)
        .await?
        .ok_or_else(|| Failure::Refused(no_base(args)))?;
    let name = project
        .free_subject_name(&slug, Zoned::now().date())
        .await?;
    let new_run = NewRun {
        subject: name.name.clone(),
        workflow: FEATURE_WORKFLOW.to_owned(),
        initial_artifacts: vec![request(&args.description), base_artifact(&base)],
        carried: Vec::new(),
        merge_into_base: merge.map(MergeFlag::mode),
        actor: project.actor().await?,
    };
    let runtime = runtime::select(project)?;
    let created_run = project.create_run(new_run).await?;
    let created_line = with_local_agents(created(&name, &base), &created_run.local_agents);
    with_merge_line(created_line, created_run.merge_into_base, merge).print_success();
    SPEC.drive(project, &name.name, &created_run.run, &runtime)
        .await
}

fn request(description: &str) -> InitialArtifact {
    single_file(
        REQUEST_ARTIFACT,
        REQUEST_FILE,
        description.as_bytes().to_vec(),
    )
}

fn short_base(base: &Commit) -> String {
    let short = base.sha.get(..SHORT_SHA_CHARS).unwrap_or(&base.sha);
    match &base.branch {
        Some(branch) => format!("{branch} @ {short}"),
        None => short.to_owned(),
    }
}

fn base_artifact(base: &Commit) -> InitialArtifact {
    let json = serde_json::json!({ "ref": base.branch, "sha": base.sha });
    single_file(BASE_ARTIFACT, BASE_FILE, json.to_string().into_bytes())
}

fn single_file(name: &str, rel_path: &str, bytes: Vec<u8>) -> InitialArtifact {
    InitialArtifact {
        name: name.to_owned(),
        files: vec![ArtifactFile {
            rel_path: rel_path.to_owned(),
            bytes,
        }],
    }
}

fn created(name: &FreeName, base: &Commit) -> Message {
    let message = Message::new(format!(
        "created {}   base: {}",
        name.name,
        short_base(base)
    ));
    if name.is_suffixed() {
        return message.context(format!("{} already exists", name.stem));
    }
    message
}

fn with_local_agents(message: Message, local_agents: &[LocalFiles]) -> Message {
    if local_agents.is_empty() {
        return message;
    }
    let agents: Vec<String> = local_agents
        .iter()
        .map(|local| format!("{} ({})", local.agent, local.files.join(", ")))
        .collect();
    message.context(format!("local agents: {}", agents.join(", ")))
}

fn no_slug(args: &FeatureArgs) -> Message {
    let headline = match &args.name {
        Some(name) => format!("cannot derive a name from --name {}", retry::quoted(name)),
        None => "cannot derive a name from the description; pass --name <slug>".to_owned(),
    };
    let fix = Retry::of(&args.description)
        .option("--base", args.base.as_deref())
        .word("--name <slug>");
    Message::new(headline).way("fix", fix.render())
}

fn active_slug(args: &FeatureArgs, active: &str) -> Message {
    let anew = Retry::of(&args.description)
        .option("--name", args.name.as_deref())
        .option("--base", args.base.as_deref())
        .word("--new");
    Message::new(format!(
        "{active} already has an active run for {}",
        retry::quoted(&args.description)
    ))
    .way("resume", format!("surt feature {active}"))
    .way("or", anew.render())
}

fn no_base(args: &FeatureArgs) -> Message {
    let Some(base) = &args.base else {
        return Message::new("HEAD does not name a commit; the repository has no commits yet")
            .way("fix", "git commit --allow-empty -m \"first commit\"");
    };
    let fix = Retry::of(&args.description)
        .option("--name", args.name.as_deref())
        .word("--base <branch, tag or commit>");
    Message::new(format!("--base {base} does not name a commit")).way("fix", fix.render())
}
