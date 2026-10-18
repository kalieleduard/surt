use std::io;
use std::num::NonZeroU32;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::error::{Error, Refusal};

mod check;
mod cleanup;
pub(crate) mod global;

pub use check::Check;
pub(crate) use check::parse_timeout;
pub use cleanup::CleanupConfig;
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
    pub cleanup: CleanupConfig,
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
            cleanup: CleanupConfig::default(),
        }
    }
}

pub(crate) fn load(path: &Path) -> Result<Config, Error> {
    read(path)
}

pub(crate) fn live_max_concurrent_agents(path: &Path) -> Result<Option<NonZeroU32>, Unreadable> {
    let present = read_if_present(path).map_err(|error| Unreadable {
        line: None,
        reason: error.to_string(),
    })?;
    let Some(text) = present else {
        return Ok(None);
    };
    let config: Config = parse(&text)?;
    Ok(Some(config.max_concurrent_agents))
}

#[derive(Debug)]
pub(crate) struct Unreadable {
    pub(crate) line: Option<usize>,
    pub(crate) reason: String,
}

pub(super) fn read<T: DeserializeOwned + Default>(path: &Path) -> Result<T, Error> {
    let present = read_if_present(path).map_err(|error| Error::io("read", path, error))?;
    let Some(text) = present else {
        return Ok(T::default());
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

fn read_if_present(path: &Path) -> io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
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
