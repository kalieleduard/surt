use std::collections::HashSet;
use std::time::Duration;

use surt_engine::project::{LogSource, Logs, Project};

use super::source::{Files, Offsets, aligned, render_with};
use super::stream::Stream;
use super::{Target, emit, missing};
use crate::failure::Failure;
use crate::message::Mark;

const POLL: Duration = Duration::from_millis(200);

struct Tail {
    source: LogSource,
    stream: Stream,
    offsets: Offsets,
}

impl Tail {
    fn new(source: LogSource) -> Tail {
        Tail {
            source,
            stream: Stream::default(),
            offsets: Offsets::default(),
        }
    }

    fn drain(&mut self, prefix: &str, raw: bool, last: bool) -> Result<String, Failure> {
        let files = Files::appended(&self.source.directory, &mut self.offsets, last)?;
        Ok(render_with(&mut self.stream, &files, prefix, raw))
    }
}

struct Follower<'a> {
    project: &'a Project,
    prefixed: bool,
    raw: bool,
    known: HashSet<String>,
    tails: Vec<Tail>,
}

impl Follower<'_> {
    fn track(&mut self, executions: Vec<LogSource>) {
        for source in executions {
            if self.known.insert(source.execution.clone()) {
                self.tails.push(Tail::new(source));
            }
        }
    }

    fn prefixes(&self) -> Vec<String> {
        if !self.prefixed {
            return vec![String::new(); self.tails.len()];
        }
        aligned(
            &self
                .tails
                .iter()
                .map(|tail| &tail.source)
                .collect::<Vec<_>>(),
        )
    }

    fn drain(&mut self, last: bool) -> Result<String, Failure> {
        let mut text = String::new();
        let prefixes = self.prefixes();
        for (tail, prefix) in self.tails.iter_mut().zip(prefixes) {
            if last && !tail.source.directory.is_dir() {
                missing(self.project, &tail.source).print(Mark::Warning);
                continue;
            }
            text.push_str(&tail.drain(&prefix, self.raw, last)?);
        }
        Ok(text)
    }
}

pub(super) async fn follow(
    project: &Project,
    target: Target,
    mut logs: Logs,
    raw: bool,
) -> Result<(), Failure> {
    let mut follower = Follower {
        project,
        prefixed: !matches!(target, Target::Execution(_)),
        raw,
        known: HashSet::new(),
        tails: Vec::new(),
    };
    let target = target.pinned(&logs);
    follower.track(std::mem::take(&mut logs.sources));
    let started = logs
        .executions
        .iter()
        .map(|source| source.execution.clone());
    follower.known.extend(started);
    loop {
        let last = !logs.live;
        let text = follower.drain(last)?;
        let open = text.is_empty() || emit(&text, raw)?;
        if last || !open {
            return Ok(());
        }
        tokio::time::sleep(POLL).await;
        logs = target.logs(project).await?;
        follower.track(std::mem::take(&mut logs.executions));
    }
}
