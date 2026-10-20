use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::Question;

#[derive(Serialize)]
pub(crate) struct Round<'a> {
    pub(crate) questions: &'a [Question],
    pub(crate) answers: &'a BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) note: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Unanswered {
    pub(crate) missing: Vec<String>,
    pub(crate) unknown: Vec<String>,
}

pub(crate) struct Accumulated {
    pub(crate) bytes: Vec<u8>,
    pub(crate) round: u32,
}

#[derive(Deserialize)]
struct Recorded {
    rounds: Vec<Value>,
}

impl Round<'_> {
    pub(crate) fn unanswered(&self) -> Option<Unanswered> {
        let missing: Vec<String> = self
            .questions
            .iter()
            .map(Question::id)
            .filter(|id| !self.answers.contains_key(*id))
            .map(str::to_owned)
            .collect();
        let unknown: Vec<String> = self
            .answers
            .keys()
            .filter(|id| self.questions.iter().all(|asked| asked.id() != *id))
            .cloned()
            .collect();
        (!missing.is_empty() || !unknown.is_empty()).then_some(Unanswered { missing, unknown })
    }
}

pub(crate) fn accumulate(
    previous: Option<&[u8]>,
    round: &Round<'_>,
) -> Result<Accumulated, serde_json::Error> {
    let mut rounds = match previous {
        Some(bytes) => serde_json::from_slice::<Recorded>(bytes)?.rounds,
        None => Vec::new(),
    };
    rounds.push(serde_json::to_value(round)?);
    let number = u32::try_from(rounds.len()).unwrap_or(u32::MAX);
    let mut bytes = serde_json::to_vec_pretty(&json!({ "rounds": rounds }))?;
    bytes.push(b'\n');
    Ok(Accumulated {
        bytes,
        round: number,
    })
}
