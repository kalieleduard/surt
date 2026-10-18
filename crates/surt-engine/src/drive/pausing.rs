//! The Driver's side of the pause on Transient Failure (I): it feeds the
//! pure schedule every agent ending, gives `decide()` the end of the pause,
//! and reports when launches pause and resume. The pause lives in memory: a
//! new driving command starts unpaused.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::driver::Driver;
use super::outcome::Progress;
use super::pause::{Pause, backoff};
use crate::project::Error;
use crate::runtime::AgentRuntime;

const AGENTS_PAUSED: &str = "agents.paused";
const AGENTS_RESUMED: &str = "agents.resumed";

#[derive(Debug, Default)]
pub(super) struct Pausing {
    schedule: Pause,
    code: String,
    announced: Option<i64>,
    since: Option<i64>,
}

/// How an agent Execution ended, as far as the pause cares.
pub(super) enum AgentEnd<'a> {
    Transient {
        code: &'a str,
        resets_at: Option<i64>,
    },
    Other,
}

impl<R: AgentRuntime> Driver<'_, R> {
    /// Called with every agent Execution that ends, before it is recorded,
    /// so the next tick already sees the pause.
    pub(super) fn learn(&self, end: &AgentEnd<'_>) {
        let now = unix_now();
        let Ok(mut pausing) = self.pausing.lock() else {
            return;
        };
        match *end {
            AgentEnd::Transient { code, resets_at } => {
                let before = pausing.schedule.until(now);
                pausing.schedule = pausing.schedule.failed(resets_at, now);
                if pausing.schedule.until(now) != before {
                    code.clone_into(&mut pausing.code);
                }
            }
            AgentEnd::Other => pausing.schedule = pausing.schedule.ended(),
        }
    }

    /// The end of the pause for this tick, reporting the resume once it is
    /// over.
    pub(super) async fn paused_until(&self, now: i64) -> Result<Option<i64>, Error> {
        let (until, resumed) = {
            let Ok(mut pausing) = self.pausing.lock() else {
                return Ok(None);
            };
            let until = pausing.schedule.until(now);
            let resumed = match (until, pausing.since) {
                (None, Some(since)) => {
                    pausing.since = None;
                    pausing.announced = None;
                    Some(u64::try_from(now - since).unwrap_or_default())
                }
                _ => None,
            };
            (until, resumed)
        };
        if let Some(paused_seconds) = resumed {
            let data = json!({ "paused_seconds": paused_seconds });
            self.record_agents_event(AGENTS_RESUMED, &data).await?;
            self.report(Progress::Resumed { paused_seconds });
        }
        Ok(until)
    }

    /// Reports the pause when it starts and whenever its end moves later.
    pub(super) async fn announce(&self, until: i64, now: i64, waiting: usize) -> Result<(), Error> {
        let (code, step) = {
            let Ok(mut pausing) = self.pausing.lock() else {
                return Ok(());
            };
            if pausing.announced == Some(until) {
                return Ok(());
            }
            pausing.announced = Some(until);
            pausing.since.get_or_insert(now);
            (pausing.code.clone(), pausing.schedule.backoff_step())
        };
        let mut data = Map::new();
        data.insert("until".to_owned(), json!(until));
        data.insert("code".to_owned(), json!(code));
        if let Some(step) = step {
            data.insert("backoff_step".to_owned(), json!(step));
        }
        data.insert("waiting".to_owned(), json!(waiting));
        self.record_agents_event(AGENTS_PAUSED, &Value::Object(data))
            .await?;
        self.report(Progress::Paused {
            until,
            waiting,
            backoff: step.map(|step| u64::try_from(backoff(step)).unwrap_or_default()),
        });
        Ok(())
    }

    /// How long until the pause ends, if launches are paused.
    pub(super) fn pause_left(&self) -> Option<Duration> {
        let until = self.pausing.lock().ok()?.schedule.until(unix_now())?;
        let end = UNIX_EPOCH + Duration::from_secs(u64::try_from(until).ok()?);
        Some(end.duration_since(SystemTime::now()).unwrap_or_default())
    }

    async fn record_agents_event(&self, kind: &str, data: &Value) -> Result<(), Error> {
        let root = self.driving.root.as_str();
        Ok(self
            .driving
            .store
            .record_run_event(root, kind, data)
            .await?)
    }
}

pub(super) fn unix_now() -> i64 {
    jiff::Timestamp::now().as_second()
}
