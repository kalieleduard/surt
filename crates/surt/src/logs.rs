mod follow;
pub(crate) mod source;
pub(crate) mod stream;

use std::io::{self, Write};
use std::path::{Component, Path};

use surt_engine::project::{LogRequest, LogSource, Logs, Project, Reference, RunSelector};

use crate::failure::Failure;
use crate::message::{Mark, Message};
use crate::status::run_selector;
use crate::symbols::printable;
use source::{Files, Format, prefixes, render};

#[derive(clap::Args)]
pub(crate) struct LogsArgs {
    subject: String,
    item: Option<String>,
    #[arg(long, value_name = "N")]
    run: Option<u32>,
    #[arg(short = 'f', long)]
    follow: bool,
    #[arg(long)]
    raw: bool,
}

pub(crate) enum Target {
    Execution(String),
    Run {
        subject: String,
        item: Option<String>,
        run: RunSelector,
    },
}

impl Target {
    async fn select(
        project: &Project,
        reference: &str,
        item: Option<&str>,
        run: Option<u32>,
    ) -> Result<Target, Failure> {
        if let (None, None) = (item, run)
            && let Reference::Execution(id) = project.reference(reference).await?
        {
            return Ok(Target::Execution(id));
        }
        Ok(Target::Run {
            subject: reference.to_owned(),
            item: item.map(str::to_owned),
            run: run_selector(run),
        })
    }

    fn pinned(self, logs: &Logs) -> Target {
        match self {
            Target::Run { item, .. } => Target::Run {
                subject: logs.subject.clone(),
                item,
                run: RunSelector::Ordinal(logs.run),
            },
            execution @ Target::Execution(_) => execution,
        }
    }

    async fn logs(&self, project: &Project) -> Result<Logs, Failure> {
        Ok(match self {
            Target::Execution(id) => project.execution_logs(id).await?,
            Target::Run { subject, item, run } => {
                let request = LogRequest {
                    item: item.as_deref(),
                    run: *run,
                };
                project.logs(subject, request).await?
            }
        })
    }
}

pub(crate) async fn run(cwd: &Path, args: LogsArgs) -> Result<(), Failure> {
    let project = Project::open_read_only(cwd).await?;
    let target = Target::select(&project, &args.subject, args.item.as_deref(), args.run).await?;
    let logs = target.logs(&project).await?;
    let format = if args.raw {
        Format::Raw
    } else {
        Format::Rendered
    };
    if args.follow && logs.live {
        return follow::follow(&project, target, logs, format).await;
    }
    print(&project, &logs.sources, format)
}

fn print(project: &Project, sources: &[LogSource], format: Format) -> Result<(), Failure> {
    let found = read(project, sources)?;
    let shown: Vec<&LogSource> = found.iter().map(|(source, _)| *source).collect();
    let text: String = found
        .iter()
        .zip(prefixes(&shown))
        .map(|((_, files), prefix)| render(files, &prefix, format))
        .collect();
    emit(&text, format).map(|_| ())
}

fn read<'a>(
    project: &Project,
    sources: &'a [LogSource],
) -> Result<Vec<(&'a LogSource, Files)>, Failure> {
    let mut found = Vec::new();
    for source in sources {
        match Files::read(source)? {
            Some(files) => found.push((source, files)),
            None if sources.len() == 1 => return Err(Failure::Refused(missing(project, source))),
            None => missing(project, source).print(Mark::Warning),
        }
    }
    Ok(found)
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

fn emit(text: &str, format: Format) -> Result<bool, Failure> {
    if format == Format::Raw {
        write(text.as_bytes())
    } else {
        write(printable(text).as_bytes())
    }
}

fn write(bytes: &[u8]) -> Result<bool, Failure> {
    let mut stdout = io::stdout().lock();
    match stdout.write_all(bytes).and_then(|()| stdout.flush()) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(false),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests;
