use std::path::PathBuf;

use super::resolving::resolve_execution_in;
use super::show::item_run;
use super::{Error, Project, RunSelector};
use crate::platform;
use crate::store::{ExecutionScopeRow, LogSourceRow, RootRunRow, Store};

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
    pub executions: Vec<LogSource>,
    pub live: bool,
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

    pub async fn execution_logs(&self, reference: &str) -> Result<Logs, Error> {
        let store = self.reader().await?;
        let logs = self.read_execution_logs(&store, reference).await;
        store.close().await;
        logs
    }

    async fn read_execution_logs(&self, store: &Store, reference: &str) -> Result<Logs, Error> {
        let source = self.source(read_execution(store, reference).await?);
        let selector = RunSelector::Ordinal(source.run);
        let root = self.select_run(store, &source.subject, selector).await?;
        Ok(Logs {
            subject: source.subject.clone(),
            run: source.run,
            item: source.item.clone(),
            live: live(&root),
            executions: vec![source.clone()],
            sources: vec![source],
        })
    }

    async fn read_logs(
        &self,
        store: &Store,
        subject: String,
        request: LogRequest<'_>,
    ) -> Result<Logs, Error> {
        let root = self.select_run(store, &subject, request.run).await?;
        let live = live(&root);
        let run = match request.item {
            None => None,
            Some(item) => Some(item_run(store, &subject, &root.id, item).await?),
        };
        let rows = store.log_sources(&root.id, run.as_deref()).await?;
        let executions = self.with_a_log(rows);
        Ok(Logs {
            sources: latest_per_run(&executions),
            executions: executions.into_iter().map(|(_, source)| source).collect(),
            subject,
            run: root.ordinal(),
            item: request.item.map(str::to_owned),
            live,
        })
    }

    fn with_a_log(&self, rows: Vec<LogSourceRow>) -> Vec<(String, LogSource)> {
        rows.into_iter()
            .map(|row| (row.run, row.kind, self.source(row.scope)))
            .filter(|(_, kind, source)| kind == AGENT || source.directory.is_dir())
            .map(|(run, _, source)| (run, source))
            .collect()
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

fn latest_per_run(executions: &[(String, LogSource)]) -> Vec<LogSource> {
    let mut latest: Vec<(&str, &LogSource)> = Vec::new();
    for (run, source) in executions {
        match latest.last_mut() {
            Some((last_run, last)) if last_run == run => *last = source,
            _ => latest.push((run, source)),
        }
    }
    latest
        .into_iter()
        .map(|(_, source)| source.clone())
        .collect()
}

fn live(root: &RootRunRow) -> bool {
    root.driver()
        .is_some_and(|driver| platform::is_alive(&driver))
}

fn number(value: i64) -> u32 {
    u32::try_from(value).unwrap_or_default()
}
