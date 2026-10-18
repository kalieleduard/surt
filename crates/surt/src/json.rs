mod base64;

use std::io::{self, Write};

use jiff::Timestamp;
use serde_json::{Map, Value};

pub(crate) use base64::encode as base64;

use crate::failure::Failure;

const SCHEMA: u32 = 1;

#[derive(Default)]
pub(crate) struct Fields(Map<String, Value>);

impl Fields {
    pub(crate) fn new() -> Fields {
        Fields::default()
    }

    pub(crate) fn with(mut self, name: &str, value: impl Into<Value>) -> Fields {
        self.0.insert(name.to_owned(), value.into());
        self
    }

    pub(crate) fn maybe(self, name: &str, value: Option<impl Into<Value>>) -> Fields {
        match value {
            Some(value) => self.with(name, value),
            None => self,
        }
    }

    pub(crate) fn time(self, name: &str, seconds: Option<i64>) -> Fields {
        self.maybe(name, seconds.map(timestamp))
    }

    pub(crate) fn list<T: Into<Value>>(self, name: &str, values: Vec<T>) -> Fields {
        if values.is_empty() {
            return self;
        }
        self.with(name, values)
    }
}

impl From<Fields> for Value {
    fn from(fields: Fields) -> Value {
        Value::Object(fields.0)
    }
}

pub(crate) fn timestamp(seconds: i64) -> Value {
    match Timestamp::from_second(seconds) {
        Ok(timestamp) => Value::String(timestamp.to_string()),
        Err(_) => Value::from(seconds),
    }
}

pub(crate) fn print(document: Fields) -> Result<(), Failure> {
    let mut fields = Fields::new().with("schema", SCHEMA);
    fields.0.extend(document.0);
    let text = serde_json::to_string_pretty(&Value::from(fields)).map_err(io::Error::from)?;
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{text}")?;
    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests;
