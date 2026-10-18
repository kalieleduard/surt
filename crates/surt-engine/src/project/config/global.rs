use std::path::Path;
use std::time::Duration;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer};

use super::{parse_timeout, read};
use crate::project::error::Error;

const DEFAULT_STOP_GRACE_PERIOD: Duration = Duration::from_secs(10);

/// The machine's `~/.surt/config.toml`: read by each process when it opens
/// a project, never frozen into runs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GlobalConfig {
    /// How long a graceful stop of an agent process tree waits before a
    /// forced kill.
    #[serde(deserialize_with = "grace_period")]
    pub stop_grace_period: Duration,
}

impl Default for GlobalConfig {
    fn default() -> GlobalConfig {
        GlobalConfig {
            stop_grace_period: DEFAULT_STOP_GRACE_PERIOD,
        }
    }
}

pub(crate) fn load(path: &Path) -> Result<GlobalConfig, Error> {
    read(path)
}

fn grace_period<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
    let text = String::deserialize(deserializer)?;
    parse_timeout(&text).ok_or_else(|| {
        D::Error::custom(format!(
            "stop_grace_period {text:?} must be a positive whole number with a unit: ms, s, m or h"
        ))
    })
}

#[cfg(test)]
mod tests;
