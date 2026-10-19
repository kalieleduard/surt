use std::fmt;

use serde::de::{Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

use super::InputSpec;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DeclaredInputs(Vec<(String, InputSpec)>);

struct InOrder;

impl DeclaredInputs {
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &InputSpec)> {
        self.0.iter().map(|(name, spec)| (name.as_str(), spec))
    }

    pub(crate) fn get(&self, name: &str) -> Option<&InputSpec> {
        self.iter()
            .find(|(declared, _)| *declared == name)
            .map(|(_, spec)| spec)
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'de> Deserialize<'de> for DeclaredInputs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(InOrder)
    }
}

impl<'de> Visitor<'de> for InOrder {
    type Value = DeclaredInputs;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a table of inputs")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<DeclaredInputs, A::Error> {
        let mut inputs = Vec::new();
        while let Some(entry) = map.next_entry::<String, InputSpec>()? {
            inputs.push(entry);
        }
        Ok(DeclaredInputs(inputs))
    }
}

impl Serialize for DeclaredInputs {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, spec) in &self.0 {
            map.serialize_entry(name, spec)?;
        }
        map.end()
    }
}
