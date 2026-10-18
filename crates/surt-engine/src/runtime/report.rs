use serde_json::{Map, Value};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuntimeReport {
    pub(crate) session_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) effort: Option<String>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) turns: Option<u64>,
    pub(crate) usage: Option<Map<String, Value>>,
    pub(crate) cost_usd: Option<f64>,
    pub(crate) permission_denials: Option<Vec<Value>>,
}

impl RuntimeReport {
    pub(crate) fn fields(&self) -> Value {
        let fields = [
            ("session_id", self.session_id.clone().map(Value::from)),
            ("model", self.model.clone().map(Value::from)),
            ("effort", self.effort.clone().map(Value::from)),
            ("exit_code", self.exit_code.map(Value::from)),
            ("turns", self.turns.map(Value::from)),
            ("usage", self.usage.clone().map(Value::Object)),
            ("cost_usd", self.cost_usd.map(Value::from)),
            (
                "permission_denials",
                self.permission_denials.clone().map(Value::Array),
            ),
        ];
        Value::Object(
            fields
                .into_iter()
                .filter_map(|(name, value)| Some((name.to_owned(), value?)))
                .filter(|(_, value)| !value.is_null())
                .collect(),
        )
    }
}
