use std::path::PathBuf;

use super::resolving::resolve_execution_in;
use super::show::item_run;
use super::{Error, Project, RunSelector};
use crate::store::{ExecutionScopeRow, LogSourceRow, Store};

const AGENT: &str = "agent";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LogRequest<'a> {
    pub item: Option<&'a str>,
    pub run: RunSelector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Logs {
    pub subject: String,
    pub run: u32,
    pub item: Option<String>,
    pub sources: Vec<LogSource>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogSource {
    pub execution: String,
    pub subject: String,
    pub run: u32,
    pub item: Option<String>,
    pub step: String,
    pub iteration: Option<u32>,
    pub attempt: u32,
    pub state: String,
    pub directory: PathBuf,
}

impl Project {
    pub async fn logs(&self, reference: &str, request: LogRequest<'_>) -> Result<Logs, Error> {
        let subject = self.resolve_subject(reference).await?.name;
        let store = self.reader().await?;
        let logs = self.read_logs(&store, subject, request).await;
        store.close().await;
        logs
    }

    pub async fn execution_logs(&self, reference: &str) -> Result<LogSource, Error> {
        let store = self.reader().await?;
        let row = read_execution(&store, reference).await;
        store.close().await;
        Ok(self.source(row?))
    }

    async fn read_logs(
        &self,
        store: &Store,
        subject: String,
        request: LogRequest<'_>,
    ) -> Result<Logs, Error> {
        let root = self.select_run(store, &subject, request.run).await?;
        let run = match request.item {
            None => None,
            Some(item) => Some(item_run(store, &subject, &root.id, item).await?),
        };
        let rows = store.log_sources(&root.id, run.as_deref()).await?;
        Ok(Logs {
            sources: self.latest_per_run(rows),
            subject,
            run: root.ordinal(),
            item: request.item.map(str::to_owned),
        })
    }

    fn latest_per_run(&self, rows: Vec<LogSourceRow>) -> Vec<LogSource> {
        let mut latest: Vec<(String, LogSource)> = Vec::new();
        for row in rows {
            let source = self.source(row.scope);
            if row.kind != AGENT && !source.directory.is_dir() {
                continue;
            }
            match latest.last_mut() {
                Some((run, last)) if *run == row.run => *last = source,
                _ => latest.push((row.run, source)),
            }
        }
        latest.into_iter().map(|(_, source)| source).collect()
    }

    fn source(&self, row: ExecutionScopeRow) -> LogSource {
        LogSource {
            directory: self.layout.execution_logs(&row.subject, &row.id),
            execution: row.id,
            subject: row.subject,
            run: number(row.ordinal),
            item: row.item,
            step: row.step,
            iteration: row.iteration.map(number),
            attempt: number(row.attempt),
            state: row.state,
        }
    }
}

async fn read_execution(store: &Store, reference: &str) -> Result<ExecutionScopeRow, Error> {
    let id = resolve_execution_in(store, reference).await?;
    let rows = store.executions_starting_with(&id).await?;
    Ok(rows
        .into_iter()
        .find(|row| row.id == id)
        .ok_or(sqlx::Error::RowNotFound)?)
}

fn number(value: i64) -> u32 {
    u32::try_from(value).unwrap_or_default()
}
