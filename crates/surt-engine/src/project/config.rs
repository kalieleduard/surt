use std::io;
use std::num::NonZeroU32;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::error::{Error, Refusal};

mod check;
pub(crate) mod global;

pub use check::Check;
pub(crate) use check::parse_timeout;
pub use global::GlobalConfig;

pub(crate) const TEMPLATE: &str = include_str!("../../assets/config.toml");

const DEFAULT_MAX_CONCURRENT_AGENTS: NonZeroU32 = NonZeroU32::new(3).unwrap();
const DEFAULT_MAX_ATTEMPTS: NonZeroU32 = NonZeroU32::new(3).unwrap();
const DEFAULT_MAX_ITERATIONS: NonZeroU32 = NonZeroU32::new(5).unwrap();

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub max_concurrent_agents: NonZeroU32,
    pub max_attempts: NonZeroU32,
    pub max_iterations: NonZeroU32,
    pub git: GitConfig,
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GitConfig {
    pub merge_into_base: MergeIntoBase,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MergeIntoBase {
    #[default]
    Never,
    FfOnly,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            max_concurrent_agents: DEFAULT_MAX_CONCURRENT_AGENTS,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            git: GitConfig::default(),
            checks: Vec::new(),
        }
    }
}

pub(crate) fn load(path: &Path) -> Result<Config, Error> {
    let config: Config = read(path)?;
    if config.git.merge_into_base == MergeIntoBase::FfOnly {
        return Err(Refusal::MergeIntoBaseUnsupported {
            path: path.to_path_buf(),
        }
        .into());
    }
    Ok(config)
}

/// Rereads `max_concurrent_agents` from the project configuration while a
/// Driver runs; `None` when the file does not exist. Only this key is read
/// live, so a file that parses is enough, whatever its other keys say.
pub(crate) fn live_max_concurrent_agents(path: &Path) -> Result<Option<NonZeroU32>, Unreadable> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Unreadable {
                line: None,
                reason: error.to_string(),
            });
        }
    };
    let config: Config = parse(&text)?;
    Ok(Some(config.max_concurrent_agents))
}

/// Why a configuration file gave no values: the line at fault, when known, and the reason.
#[derive(Debug)]
pub(crate) struct Unreadable {
    pub(crate) line: Option<usize>,
    pub(crate) reason: String,
}

/// Reads a configuration file, or its defaults when it does not exist.
/// A file that does not parse is refused with its path, line and reason.
fn read<T: DeserializeOwned + Default>(path: &Path) -> Result<T, Error> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(T::default()),
        Err(error) => return Err(Error::io("read", path, error)),
    };
    parse(&text).map_err(|unreadable| {
        Refusal::InvalidConfig {
            path: path.to_path_buf(),
            line: unreadable.line,
            reason: unreadable.reason,
        }
        .into()
    })
}

fn parse<T: DeserializeOwned>(text: &str) -> Result<T, Unreadable> {
    toml::from_str(text).map_err(|error: toml::de::Error| Unreadable {
        line: error.span().map(|span| line_of(text, span.start)),
        reason: error.message().to_owned(),
    })
}

fn line_of(text: &str, offset: usize) -> usize {
    text.get(..offset)
        .map_or(0, |before| before.matches('\n').count())
        + 1
}

#[cfg(test)]
mod tests;
