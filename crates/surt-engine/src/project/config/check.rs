use std::time::Duration;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

const TIMEOUT_UNITS: [(&str, u64); 4] = [("ms", 1), ("s", 1_000), ("m", 60_000), ("h", 3_600_000)];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    #[serde(deserialize_with = "identifier")]
    pub name: String,
    #[serde(deserialize_with = "argv")]
    pub run: Vec<String>,
    #[serde(
        default,
        deserialize_with = "timeout",
        skip_serializing_if = "Option::is_none"
    )]
    pub timeout: Option<String>,
}

pub(crate) fn parse_timeout(text: &str) -> Option<Duration> {
    TIMEOUT_UNITS.iter().find_map(|(unit, millis)| {
        let digits = text.strip_suffix(unit)?;
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let amount: u64 = digits.parse().ok()?;
        let millis = amount.checked_mul(*millis).filter(|millis| *millis > 0)?;
        Some(Duration::from_millis(millis))
    })
}

fn identifier<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let name = String::deserialize(deserializer)?;
    let mut characters = name.chars();
    let valid = characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|rest| rest.is_ascii_lowercase() || rest.is_ascii_digit() || rest == '-');
    if !valid {
        return Err(D::Error::custom(format!(
            "check name {name:?} must be lowercase letters, digits and hyphens, starting with a letter"
        )));
    }
    Ok(name)
}

fn argv<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let run = Vec::<String>::deserialize(deserializer)?;
    if run.first().is_none_or(String::is_empty) {
        return Err(D::Error::custom(
            "check run must name a program followed by its arguments",
        ));
    }
    Ok(run)
}

fn timeout<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    let text = String::deserialize(deserializer)?;
    if parse_timeout(&text).is_none() {
        return Err(D::Error::custom(format!(
            "check timeout {text:?} must be a positive whole number with a unit: ms, s, m or h"
        )));
    }
    Ok(Some(text))
}
