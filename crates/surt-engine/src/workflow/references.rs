use serde_json::Value;

const REFERENCE_KEY: &str = "$ref";
const FRAGMENT: char = '#';
const SCHEME: char = ':';
const SEPARATOR: char = '/';
const CURRENT: &str = ".";
const PARENT: &str = "..";

pub(super) enum Reference {
    Local(String),
    Outside(String),
}

pub(crate) fn local_references(path: &str, text: &str) -> Vec<String> {
    references(path, text)
        .into_iter()
        .filter_map(|reference| match reference {
            Reference::Local(path) => Some(path),
            Reference::Outside(_) => None,
        })
        .collect()
}

pub(super) fn references(path: &str, text: &str) -> Vec<Reference> {
    let Ok(schema) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let mut references = Vec::new();
    collect(&schema, &mut references);
    let directory = path.rsplit_once(SEPARATOR).map_or("", |(dir, _)| dir);
    references
        .into_iter()
        .filter_map(|reference| {
            let (file, _) = reference.split_once(FRAGMENT).unwrap_or((reference, ""));
            if file.is_empty() {
                return None;
            }
            let local = !file.contains(SCHEME) && !file.starts_with(SEPARATOR);
            Some(
                local
                    .then(|| resolve(directory, file))
                    .flatten()
                    .map_or_else(
                        || Reference::Outside(reference.to_owned()),
                        Reference::Local,
                    ),
            )
        })
        .collect()
}

fn collect<'a>(value: &'a Value, references: &mut Vec<&'a str>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                match (key.as_str(), value) {
                    (REFERENCE_KEY, Value::String(reference)) => references.push(reference),
                    _ => collect(value, references),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect(item, references)),
        _ => {}
    }
}

fn resolve(directory: &str, file: &str) -> Option<String> {
    let mut parts: Vec<&str> = directory
        .split(SEPARATOR)
        .filter(|part| !part.is_empty())
        .collect();
    for part in file.split(SEPARATOR) {
        match part {
            "" | CURRENT => {}
            PARENT => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}
