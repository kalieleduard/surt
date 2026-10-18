use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::decide::{Ctx, StopLevel};
use super::driver::Driver;
use super::outcome::{ActivityPause, Progress};
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

#[derive(Debug, PartialEq, Eq)]
pub(super) enum AgentEnd<'a> {
    Transient {
        code: &'a str,
        resets_at: Option<i64>,
        launched_at: i64,
    },
    Other,
}

impl Pausing {
    fn learn(&mut self, end: &AgentEnd<'_>, now: i64) {
        match *end {
            AgentEnd::Transient {
                code,
                resets_at,
                launched_at,
            } => {
                let before = self.schedule.until(now);
                self.schedule = self.schedule.failed(launched_at, resets_at, now);
                if self.schedule.until(now) != before {
                    code.clone_into(&mut self.code);
                }
            }
            AgentEnd::Other => self.schedule = self.schedule.ended(),
        }
    }

    fn check(&mut self, now: i64) -> (Option<i64>, Option<u64>) {
        let until = self.schedule.until(now);
        if until.is_some() {
            return (until, None);
        }
        self.announced = None;
        let lasted = self.since.take().map(|since| now - since);
        (
            None,
            lasted.map(|seconds| u64::try_from(seconds).unwrap_or_default()),
        )
    }

    fn announce(&mut self, until: i64, now: i64) -> Option<(String, Option<u32>)> {
        if self.announced == Some(until) {
            return None;
        }
        self.announced = Some(until);
        self.since.get_or_insert(now);
        Some((self.code.clone(), self.schedule.backoff_step()))
    }
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) fn learn(&self, end: &AgentEnd<'_>) {
        self.with_pausing(|pausing| pausing.learn(end, unix_now()));
    }

    pub(super) async fn paused_until(&self, now: i64) -> Result<Option<i64>, Error> {
        let (until, resumed) = self
            .with_pausing(|pausing| pausing.check(now))
            .unwrap_or_default();
        if let Some(paused_seconds) = resumed {
            let data = json!({ "paused_seconds": paused_seconds });
            self.record_agents_event(AGENTS_RESUMED, &data).await?;
            self.report(Progress::Resumed { paused_seconds });
        }
        Ok(until)
    }

    pub(super) async fn announce(&self, ctx: &Ctx, waiting: usize) -> Result<(), Error> {
        let (Some(until), StopLevel::None, 1..) = (ctx.paused_until, ctx.stop, waiting) else {
            return Ok(());
        };
        let announced = self.with_pausing(|pausing| pausing.announce(until, ctx.now));
        let Some((code, step)) = announced.flatten() else {
            return Ok(());
        };
        let data = paused(until, &code, step, waiting);
        self.record_agents_event(AGENTS_PAUSED, &data).await?;
        self.report(Progress::Paused {
            until,
            waiting,
            backoff: step.map(backoff_seconds),
        });
        Ok(())
    }

    pub(super) fn activity_pause(&self, until: Option<i64>) -> Option<ActivityPause> {
        let step = self.with_pausing(|pausing| pausing.schedule.backoff_step())?;
        Some(ActivityPause {
            until: until?,
            backoff: step.map(backoff_seconds),
        })
    }

    pub(super) fn pause_left(&self) -> Option<Duration> {
        let until = self.with_pausing(|pausing| pausing.schedule.until(unix_now()))??;
        let end = UNIX_EPOCH + Duration::from_secs(u64::try_from(until).ok()?);
        Some(end.duration_since(SystemTime::now()).unwrap_or_default())
    }

    fn with_pausing<T>(&self, change: impl FnOnce(&mut Pausing) -> T) -> Option<T> {
        let mut pausing = self.pausing.lock().ok()?;
        Some(change(&mut pausing))
    }

    async fn record_agents_event(&self, kind: &str, data: &Value) -> Result<(), Error> {
        let (store, root) = (self.driving.store, self.driving.root.as_str());
        Ok(store.record_run_event(root, kind, data).await?)
    }
}

fn backoff_seconds(step: u32) -> u64 {
    u64::try_from(backoff(step)).unwrap_or_default()
}

fn paused(until: i64, code: &str, step: Option<u32>, waiting: usize) -> Value {
    let mut data = Map::new();
    data.insert("until".to_owned(), json!(until));
    data.insert("code".to_owned(), json!(code));
    if let Some(step) = step {
        data.insert("backoff_step".to_owned(), json!(step));
    }
    data.insert("waiting".to_owned(), json!(waiting));
    Value::Object(data)
}
pub(super) fn unix_now() -> i64 {
    jiff::Timestamp::now().as_second()
}
