use std::io::Write;
use std::path::Path;

use surt_engine::project::{ArtifactSummary, Project, RevisionOrigin, ShowRequest};

use crate::failure::Failure;
use crate::message::{Mark, Message};
use crate::status::run_selector;
use crate::symbols::{SEPARATOR, SUCCESS, printable};
use crate::table;

const HEADERS: [&str; 3] = ["ARTIFACT", "REVS", "LATEST"];

#[derive(clap::Args)]
pub(crate) struct ShowArgs {
    feature: String,
    artifact: Option<String>,
    #[arg(long, value_name = "N", requires = "artifact")]
    rev: Option<u32>,
    #[arg(long, value_name = "key", requires = "artifact")]
    task: Option<String>,
    #[arg(long, value_name = "N", requires = "task", conflicts_with = "rev")]
    iter: Option<u32>,
    #[arg(long, value_name = "path", requires = "artifact")]
    file: Option<String>,
    #[arg(long, value_name = "N")]
    run: Option<u32>,
}

pub(crate) async fn run(cwd: &Path, args: ShowArgs) -> Result<(), Failure> {
    let project = Project::open_read_only(cwd).await?;
    let Some(artifact) = &args.artifact else {
        let artifacts = project
            .artifacts(&args.feature, run_selector(args.run))
            .await?;
        anstream::print!("{}", printable(&index(&artifacts)));
        return Ok(());
    };
    let request = ShowRequest {
        artifact,
        revision: args.rev,
        item: args.task.as_deref(),
        iteration: args.iter,
        file: args.file.as_deref(),
        run: run_selector(args.run),
    };
    let shown = project.show(&args.feature, request).await?;
    for file in &shown.edited {
        Message::new(format!("Working Copy has an unrecorded edit to {file}"))
            .context(format!(
                "this is the recorded {} rev {}",
                shown.artifact, shown.revision
            ))
            .print(Mark::Warning);
    }
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&shown.content)?;
    stdout.flush()?;
    Ok(())
}

pub(crate) fn origin(origin: RevisionOrigin) -> &'static str {
    match origin {
        RevisionOrigin::Agent => "agent",
        RevisionOrigin::Human => "human",
    }
}

fn index(artifacts: &[ArtifactSummary]) -> String {
    let rows: Vec<[String; 3]> = std::iter::once(HEADERS.map(str::to_owned))
        .chain(artifacts.iter().map(row))
        .collect();
    table::render(&rows)
}

fn row(artifact: &ArtifactSummary) -> [String; 3] {
    let mut latest = format!("rev {} ({})", artifact.latest, origin(artifact.origin));
    if artifact.approved {
        latest.push_str(&format!(" {SUCCESS} approved"));
    }
    if artifact.files > 1 {
        latest.push_str(&format!("{SEPARATOR}{} files", artifact.files));
    }
    [
        artifact.name.clone(),
        artifact.revisions.to_string(),
        latest,
    ]
}
