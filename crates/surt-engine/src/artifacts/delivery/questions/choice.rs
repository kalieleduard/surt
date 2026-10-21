use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Choice {
    Plain(String),
    Described(DescribedChoice),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DescribedChoice {
    value: String,
    description: String,
}

impl Choice {
    pub fn value(&self) -> &str {
        match self {
            Choice::Plain(value) => value,
            Choice::Described(described) => &described.value,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            Choice::Plain(_) => None,
            Choice::Described(described) => Some(&described.description),
        }
    }
}
