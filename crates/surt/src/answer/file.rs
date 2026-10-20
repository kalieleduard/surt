use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::failure::Failure;
use crate::message::Message;

const EXPECTED: &str = "expected one line per question: \"<id>\" = \"<answer>\"";

pub(super) fn read(
    path: &Path,
    shown: &Path,
    inspect: String,
) -> Result<BTreeMap<String, String>, Failure> {
    let text = fs::read_to_string(path).map_err(|error| {
        Failure::Refused(
            Message::new(format!("cannot read the answers in {}", shown.display()))
                .context(error.to_string()),
        )
    })?;
    toml::from_str(&text).map_err(|error| Failure::Refused(invalid(shown, &text, &error, inspect)))
}

fn invalid(shown: &Path, text: &str, error: &toml::de::Error, inspect: String) -> Message {
    let line = error
        .span()
        .and_then(|span| text.get(..span.start))
        .map(|before| before.matches('\n').count() + 1);
    let location = match line {
        Some(line) => format!("{}:{line}", shown.display()),
        None => shown.display().to_string(),
    };
    Message::new(format!("invalid answers in {location}"))
        .context(error.message())
        .context(EXPECTED)
        .way("inspect", inspect)
}
