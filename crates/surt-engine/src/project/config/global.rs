use std::time::Duration;

use serde::{Deserialize, Deserializer};

use super::check::duration_setting;

const DEFAULT_STOP_GRACE_PERIOD: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GlobalConfig {
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

fn grace_period<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
    duration_setting("stop_grace_period", &String::deserialize(deserializer)?)
}

#[cfg(test)]
mod tests;
