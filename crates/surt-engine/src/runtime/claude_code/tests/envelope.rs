use serde_json::{Value, json};
use uuid::Uuid;

use super::executions::Execution;

const SPECCER: &str = r#"
schema = 1
id = "noter"
outcomes = ["done", "questions"]

[outputs.spec]
format = "markdown"
outcomes = ["done"]

[outputs.spec-questions]
format = "json"
outcomes = ["questions"]

[outputs.notes]
format = "text"

[outputs.log]
format = "text"
outcomes = ["questions", "done"]

[profile]
runtime = "claude-code"
model = "claude-haiku-5-5"
effort = "low"

[profile.claude]
auth = "subscription"
permission_mode = "dontAsk"
tools = ["Read", "Write"]
allowed_tools = ["Read"]
disallowed_tools = ["Bash"]
"#;

fn envelope_schema(toml: &'static str) -> String {
    let arguments = Execution::with_agent(toml, Uuid::now_v7())
        .invocation(&[])
        .arguments;
    let flag = arguments
        .iter()
        .position(|argument| argument == "--json-schema")
        .expect("a --json-schema argument");
    arguments
        .get(flag + 1)
        .expect("the schema after --json-schema")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn the_envelope_requires_only_the_outputs_every_outcome_delivers() {
    let text = envelope_schema(SPECCER);
    let schema: Value = serde_json::from_str(&text).expect("a JSON schema");

    assert_eq!(
        schema["properties"]["outcome"],
        json!({ "enum": ["done", "questions"] })
    );
    assert_eq!(
        schema["properties"]["outputs"],
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["log", "notes"],
            "properties": {
                "log": { "type": "string" },
                "notes": { "type": "string" },
                "spec": { "type": "string" },
                "spec-questions": { "type": "string" }
            }
        })
    );
    assert!(!text.contains("oneOf"), "{text}");
}
