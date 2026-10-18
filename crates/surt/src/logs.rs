pub(crate) mod source;
pub(crate) mod stream;

use std::io::{self, Write};
use std::path::{Component, Path};

use surt_engine::project::{LogRequest, LogSource, Project, Reference};

use crate::failure::Failure;
use crate::message::{Mark, Message};
use crate::status::run_selector;
use crate::symbols::printable;
use source::{Files, prefixes, render};

#[derive(clap::Args)]
pub(crate) struct LogsArgs {
    feature: String,
    task: Option<String>,
    #[arg(long, value_name = "N")]
    run: Option<u32>,
    #[arg(long)]
    raw: bool,
}

pub(crate) async fn run(cwd: &Path, args: LogsArgs) -> Result<(), Failure> {
    let project = Project::open_read_only(cwd).await?;
    let sources = select(&project, &args.feature, args.task.as_deref(), args.run).await?;
    let found = read(&project, &sources)?;
    let shown: Vec<&LogSource> = found.iter().map(|(source, _)| *source).collect();
    let text: String = found
        .iter()
        .zip(prefixes(&shown))
        .map(|((_, files), prefix)| render(files, &prefix, args.raw))
        .collect();
    if args.raw {
        write(text.as_bytes())
    } else {
        write(printable(&text).as_bytes())
    }
}

fn read<'a>(
    project: &Project,
    sources: &'a [LogSource],
) -> Result<Vec<(&'a LogSource, Files)>, Failure> {
    let mut found = Vec::new();
    for source in sources {
        match Files::read(&source.directory)? {
            Some(files) => found.push((source, files)),
            None if sources.len() == 1 => return Err(Failure::Refused(missing(project, source))),
            None => missing(project, source).print(Mark::Warning),
        }
    }
    Ok(found)
}

pub(crate) async fn select(
    project: &Project,
    reference: &str,
    task: Option<&str>,
    run: Option<u32>,
) -> Result<Vec<LogSource>, Failure> {
    if let (None, None) = (task, run)
        && let Reference::Execution(id) = project.reference(reference).await?
    {
        return Ok(vec![project.execution_logs(&id).await?]);
    }
    let request = LogRequest {
        item: task,
        run: run_selector(run),
    };
    Ok(project.logs(reference, request).await?.sources)
}

pub(crate) fn missing(project: &Project, source: &LogSource) -> Message {
    let directory = source
        .directory
        .strip_prefix(project.root())
        .unwrap_or(&source.directory);
    Message::new(format!(
        "no log for execution {} ({}/ is missing)",
        source.execution,
        slashed(directory)
    ))
    .way("inspect", format!("surt inspect {}", source.execution))
}

fn slashed(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .map(|part| match part {
            Component::Normal(name) => name.to_string_lossy().into_owned(),
            other => other.as_os_str().to_string_lossy().into_owned(),
        })
        .collect();
    parts.join("/")
}

fn write(bytes: &[u8]) -> Result<(), Failure> {
    let mut stdout = io::stdout().lock();
    match stdout.write_all(bytes).and_then(|()| stdout.flush()) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests;
