use serde_json::{Map, Value};

const SHORT_SHA: usize = 7;

#[derive(Clone, Copy)]
pub(super) struct Fields<'a>(pub(super) &'a Map<String, Value>);

impl<'a> Fields<'a> {
    pub(super) fn text(self, key: &str) -> Option<&'a str> {
        self.0.get(key)?.as_str()
    }

    pub(super) fn number(self, key: &str) -> Option<i64> {
        self.0.get(key)?.as_i64()
    }

    pub(super) fn list(self, key: &str) -> &'a [Value] {
        self.0
            .get(key)
            .and_then(Value::as_array)
            .map_or(&[], Vec::as_slice)
    }

    pub(super) fn object(self, key: &str) -> Option<Fields<'a>> {
        self.0.get(key)?.as_object().map(Fields)
    }

    pub(super) fn sha(self, key: &str) -> Option<String> {
        Some(self.text(key)?.chars().take(SHORT_SHA).collect())
    }

    pub(super) fn labelled(self, label: &str, key: &str) -> Option<String> {
        let value = self.0.get(key)?;
        let shown = match value {
            Value::String(text) => text.clone(),
            Value::Number(number) => number.to_string(),
            _ => return None,
        };
        Some(format!("{label} {shown}"))
    }
}

pub(super) fn joined(
    parts: impl IntoIterator<Item = Option<String>>,
    separator: &str,
) -> Option<String> {
    let present: Vec<String> = parts.into_iter().flatten().collect();
    if present.is_empty() {
        None
    } else {
        Some(present.join(separator))
    }
}
