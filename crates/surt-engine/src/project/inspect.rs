mod execution;

use serde_json::{Map, Value};

use super::show::item_run;
use super::{Error, Project, RunSelector};
use crate::store::{EventRow, Store};

pub use execution::{
    AgentProfile, ExecutionDetail, ExecutionFailure, ExecutionInput, Performer, RuntimeInfo, Usage,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InspectRequest<'a> {
    pub item: Option<&'a str>,
    pub run: RunSelector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timeline {
    pub subject: String,
    pub run: u32,
    pub item: Option<String>,
    pub events: Vec<TimelineEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineEvent {
    pub at: i64,
    pub kind: String,
    pub item: Option<String>,
    pub step: Option<String>,
    pub iteration: Option<u32>,
    pub attempt: Option<u32>,
    pub execution: Option<String>,
    pub data: Map<String, Value>,
}

impl Project {
    pub async fn inspect(
        &self,
        reference: &str,
        request: InspectRequest<'_>,
    ) -> Result<Timeline, Error> {
        let subject = self.resolve_subject(reference).await?.name;
        let store = self.reader().await?;
        let timeline = self.read_timeline(&store, subject, request).await;
        store.close().await;
        timeline
    }

    async fn read_timeline(
        &self,
        store: &Store,
        subject: String,
        request: InspectRequest<'_>,
    ) -> Result<Timeline, Error> {
        let root = self.select_run(store, &subject, request.run).await?;
        let run = match request.item {
            None => None,
            Some(item) => Some(item_run(store, &subject, &root.id, item).await?),
        };
        let rows = store.timeline(&root.id, run.as_deref()).await?;
        Ok(Timeline {
            subject,
            run: root.ordinal(),
            item: request.item.map(str::to_owned),
            events: rows.into_iter().map(TimelineEvent::from).collect(),
        })
    }
}

impl From<EventRow> for TimelineEvent {
    fn from(row: EventRow) -> TimelineEvent {
        TimelineEvent {
            at: row.at,
            kind: row.kind,
            item: row.item,
            step: row.step,
            iteration: row.iteration.and_then(|value| u32::try_from(value).ok()),
            attempt: row.attempt.and_then(|value| u32::try_from(value).ok()),
            execution: row.execution,
            data: match serde_json::from_str(&row.data) {
                Ok(Value::Object(data)) => data,
                _ => Map::new(),
            },
        }
    }
}
