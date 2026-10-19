mod tools;

use serde_json::{Map, Value};

use crate::symbols::{ELLIPSIS, SUCCESS};

type Event = Map<String, Value>;

const DEFAULT_OUTCOME: &str = "done";
const SHORT_SHA: usize = 7;

#[derive(Default)]
pub(crate) struct Stream {
    cwd: Option<String>,
}

impl Stream {
    pub(crate) fn render(&mut self, line: &str) -> Option<String> {
        let Ok(Value::Object(event)) = serde_json::from_str(line) else {
            return None;
        };
        match event.get("type").and_then(Value::as_str)? {
            "system" => self.initialize(&event),
            "assistant" => self.assistant(&event),
            "result" => delivered(&event),
            _ => None,
        }
    }

    fn initialize(&mut self, event: &Event) -> Option<String> {
        if text(event, "subtype") == Some("init") {
            self.cwd = text(event, "cwd").map(str::to_owned);
        }
        None
    }

    fn assistant(&self, event: &Event) -> Option<String> {
        let blocks = event.get("message")?.get("content")?.as_array()?;
        blocks
            .iter()
            .filter_map(Value::as_object)
            .find_map(|block| match text(block, "type")? {
                "text" => quoted(text(block, "text")?),
                "tool_use" => tools::call(block, self.cwd.as_deref()),
                _ => None,
            })
    }
}

fn delivered(event: &Event) -> Option<String> {
    if event.get("is_error").and_then(Value::as_bool) != Some(false) {
        return None;
    }
    let delivery = event.get("structured_output")?.as_object()?;
    let mut details = Vec::new();
    details.extend(text(delivery, "outcome").filter(|outcome| *outcome != DEFAULT_OUTCOME));
    let sha = delivery
        .get("outputs")
        .and_then(|outputs| outputs.get("commit"))
        .and_then(|commit| commit.get("sha"))
        .and_then(Value::as_str);
    let commit = sha.map(|sha| format!("commit {}", sha.get(..SHORT_SHA).unwrap_or(sha)));
    details.extend(commit.as_deref());
    Some(match details.as_slice() {
        [] => format!("{SUCCESS} delivered"),
        _ => format!("{SUCCESS} delivered ({})", details.join(", ")),
    })
}

fn quoted(message: &str) -> Option<String> {
    first_line(message).map(|line| format!("\"{line}\""))
}

pub(super) fn first_line(value: &str) -> Option<String> {
    let mut lines = value.lines().filter(|line| !line.trim().is_empty());
    let first = lines.next()?.trim();
    Some(match lines.next() {
        Some(_) => format!("{first}{ELLIPSIS}"),
        None => first.to_owned(),
    })
}

fn text<'a>(event: &'a Event, key: &str) -> Option<&'a str> {
    event.get(key).and_then(Value::as_str)
}
