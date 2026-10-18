use crate::artifacts::DeliveryContract;
use crate::workflow::{AgentAssets, AgentContract};

const SCHEMA_PATH: &str = "schemas/data.schema.json";

const AGENT: &str = r#"
schema = 1
id = "writer"

[outputs.data]
format = "json"
json_schema = "schemas/data.schema.json"

[profile]
runtime = "fake"
model = "claude-sonnet-5-5"
effort = "high"
"#;

fn build(schemas: &'static [(&'static str, &'static str)]) -> Result<String, String> {
    let assets = AgentAssets {
        toml: AGENT,
        instructions: "Write.",
        schemas,
    };
    let agent = AgentContract::parse("writer", &assets).expect("a valid agent contract");
    DeliveryContract::new(&agent, |path| assets.schema(path))
        .map(|_| String::new())
        .map_err(|invalid| invalid.path)
}

#[test]
fn a_schema_that_is_not_json_cannot_back_a_delivery_contract() {
    assert_eq!(
        build(&[(SCHEMA_PATH, "{ not json")]),
        Err(SCHEMA_PATH.to_owned())
    );
}

#[test]
fn a_schema_that_is_not_a_valid_draft_07_schema_cannot_back_a_delivery_contract() {
    assert_eq!(
        build(&[(SCHEMA_PATH, r#"{ "type": 12 }"#)]),
        Err(SCHEMA_PATH.to_owned())
    );
}
