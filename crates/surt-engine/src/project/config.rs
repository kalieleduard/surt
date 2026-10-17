use std::io;
use std::num::NonZeroU32;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::error::{Error, Refusal};

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub name: String,
    pub run: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
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
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(error) => return Err(Error::io("read", path, error)),
    };
    let config = parse(&text).map_err(|error| Refusal::InvalidConfig {
        path: path.to_path_buf(),
        line: error.span().map(|span| line_of(&text, span.start)),
        reason: error.message().to_owned(),
    })?;
    if config.git.merge_into_base == MergeIntoBase::FfOnly {
        return Err(Refusal::MergeIntoBaseUnsupported {
            path: path.to_path_buf(),
        }
        .into());
    }
    Ok(config)
}

fn parse(text: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(text)
}

fn line_of(text: &str, offset: usize) -> usize {
    text.get(..offset)
        .map_or(0, |before| before.matches('\n').count())
        + 1
}

#[cfg(test)]
mod tests;
