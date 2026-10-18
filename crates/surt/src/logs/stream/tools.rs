use serde_json::{Map, Value};

use super::first_line;
use crate::symbols::MINUS;

const DELIVERY_TOOL: &str = "StructuredOutput";
const ARGUMENTS: [&str; 9] = [
    "file_path",
    "notebook_path",
    "command",
    "pattern",
    "url",
    "query",
    "description",
    "prompt",
    "path",
];
const PATHS: [&str; 3] = ["file_path", "notebook_path", "path"];

pub(super) fn call(block: &Map<String, Value>, cwd: Option<&str>) -> Option<String> {
    let name = block.get("name")?.as_str()?;
    if name == DELIVERY_TOOL {
        return None;
    }
    let empty = Map::new();
    let input = block
        .get("input")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let mut line = name.to_owned();
    if let Some(argument) = argument(input, cwd) {
        line.push(' ');
        line.push_str(&argument);
    }
    if let Some((added, removed)) = changes(input) {
        line.push_str(&format!(" (+{added} {MINUS}{removed})"));
    }
    Some(line)
}

fn argument(input: &Map<String, Value>, cwd: Option<&str>) -> Option<String> {
    let (key, value) = ARGUMENTS
        .iter()
        .find_map(|key| Some((*key, input.get(*key)?.as_str()?)))?;
    if PATHS.contains(&key) {
        return Some(relative(value, cwd).to_owned());
    }
    first_line(value)
}

fn relative<'a>(path: &'a str, cwd: Option<&str>) -> &'a str {
    cwd.and_then(|cwd| path.strip_prefix(cwd))
        .and_then(|rest| rest.strip_prefix(['/', '\\']))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(path)
}

fn changes(input: &Map<String, Value>) -> Option<(usize, usize)> {
    match input.get("edits").and_then(Value::as_array) {
        Some(edits) => edits
            .iter()
            .filter_map(Value::as_object)
            .filter_map(edit)
            .reduce(|(added, removed), (more, fewer)| (added + more, removed + fewer)),
        None => edit(input),
    }
}

fn edit(input: &Map<String, Value>) -> Option<(usize, usize)> {
    let new = input.get("new_string")?.as_str()?;
    let old = input.get("old_string")?.as_str()?;
    Some((new.lines().count(), old.lines().count()))
}
