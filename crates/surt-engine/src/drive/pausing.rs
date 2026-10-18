use std::sync::PoisonError;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::decide::{Ctx, StopLevel};
use super::driver::Driver;
use super::outcome::{ActivityPause, Progress};
use super::pause::{PauseSchedule, backoff};
use crate::project::Error;
use crate::runtime::AgentRuntime;

const AGENTS_PAUSED: &str = "agents.paused";
const AGENTS_RESUMED: &str = "agents.resumed";

#[derive(Debug, Default)]
pub(super) struct PauseTracker {
    schedule: PauseSchedule,
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

enum PauseCheck {
    Paused { until: i64 },
    JustResumed { paused_seconds: i64 },
    Unpaused,
}

struct Announcement {
    code: String,
    backoff_step: Option<u32>,
}

impl PauseTracker {
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

    fn check(&mut self, now: i64) -> PauseCheck {
        if let Some(until) = self.schedule.until(now) {
            return PauseCheck::Paused { until };
        }
        self.announced = None;
        match self.since.take() {
            Some(since) => PauseCheck::JustResumed {
                paused_seconds: (now - since).max(0),
            },
            None => PauseCheck::Unpaused,
        }
    }

    fn announce(&mut self, until: i64, now: i64) -> Option<Announcement> {
        if self.announced == Some(until) {
            return None;
        }
        self.announced = Some(until);
        self.since.get_or_insert(now);
        Some(Announcement {
            code: self.code.clone(),
            backoff_step: self.schedule.backoff_step(),
        })
    }
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) fn learn(&self, end: &AgentEnd<'_>) {
        self.with_pause(|tracker| tracker.learn(end, unix_now()));
    }

    pub(super) async fn paused_until(&self, now: i64) -> Result<Option<i64>, Error> {
        match self.with_pause(|tracker| tracker.check(now)) {
            PauseCheck::Paused { until } => Ok(Some(until)),
            PauseCheck::JustResumed { paused_seconds } => {
                let data = json!({ "paused_seconds": paused_seconds });
                self.record_agents_event(AGENTS_RESUMED, &data).await?;
                self.report(Progress::Resumed { paused_seconds });
                Ok(None)
            }
            PauseCheck::Unpaused => Ok(None),
        }
    }

    pub(super) async fn announce(&self, ctx: &Ctx, waiting: usize) -> Result<(), Error> {
        let (Some(until), StopLevel::None, 1..) = (ctx.paused_until, ctx.stop, waiting) else {
            return Ok(());
        };
        let Some(announcement) = self.with_pause(|tracker| tracker.announce(until, ctx.now)) else {
            return Ok(());
        };
        let data = paused_event_data(until, &announcement, waiting);
        self.record_agents_event(AGENTS_PAUSED, &data).await?;
        self.report(Progress::Paused {
            pause: ActivityPause {
                until,
                backoff: announcement.backoff_step.map(backoff),
            },
            waiting,
        });
        Ok(())
    }

    pub(super) fn activity_pause(&self, until: Option<i64>) -> Option<ActivityPause> {
        let step = self.with_pause(|tracker| tracker.schedule.backoff_step());
        Some(ActivityPause {
            until: until?,
            backoff: step.map(backoff),
        })
    }

    pub(super) fn pause_left(&self) -> Option<Duration> {
        let until = self.with_pause(|tracker| tracker.schedule.until(unix_now()))?;
        let end = UNIX_EPOCH + Duration::from_secs(u64::try_from(until).ok()?);
        Some(end.duration_since(SystemTime::now()).unwrap_or_default())
    }

    fn with_pause<T>(&self, change: impl FnOnce(&mut PauseTracker) -> T) -> T {
        let mut tracker = self.pause.lock().unwrap_or_else(PoisonError::into_inner);
        change(&mut tracker)
    }

    async fn record_agents_event(&self, kind: &str, data: &Value) -> Result<(), Error> {
        let (store, root) = (self.driving.store, self.driving.root.as_str());
        Ok(store.record_run_event(root, kind, data).await?)
    }
}

fn paused_event_data(until: i64, announcement: &Announcement, waiting: usize) -> Value {
    let mut data = Map::new();
    data.insert("until".to_owned(), json!(until));
    data.insert("code".to_owned(), json!(announcement.code));
    if let Some(step) = announcement.backoff_step {
        data.insert("backoff_step".to_owned(), json!(step));
    }
    data.insert("waiting".to_owned(), json!(waiting));
    Value::Object(data)
}

pub(super) fn unix_now() -> i64 {
    jiff::Timestamp::now().as_second()
}
