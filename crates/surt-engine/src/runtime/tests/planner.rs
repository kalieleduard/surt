use super::fixtures::{Execution, contract};
use crate::artifacts::{DeliveryContract, DeliveryRejection, ItemListProblem};
use crate::runtime::Delivery;

const PLANNER_AGENT: &str = r#"
schema = 1
id = "planner"

[outputs.manifest]
format = "item_list"
json_schema = "schemas/manifest.schema.json"

[outputs.manifest.item_files.task]
format = "markdown"

[profile]
runtime = "fake"
model = "claude-sonnet-5-5"
effort = "high"
"#;

const MANIFEST_SCHEMA: &str = r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "items": { "type": "object", "required": ["key", "title"] }
}"#;

pub(super) const TWO_TASKS: &str = r#"[
  { "key": "A", "needs": [], "title": "Create users table", "files": { "task": "tasks/A.md" } },
  { "key": "B", "needs": ["A"], "title": "Login endpoint", "files": { "task": "tasks/B.md" } }
]"#;

pub(super) fn planner_contract() -> DeliveryContract {
    contract(
        "planner",
        PLANNER_AGENT,
        &[("schemas/manifest.schema.json", MANIFEST_SCHEMA)],
    )
}

pub(super) fn manifest(index: &str) -> Delivery {
    Delivery::new("done")
        .output("manifest", "manifest.json", index)
        .file("tasks/A.md", "# A\n")
        .file("tasks/B.md", "# B\n")
}

pub(super) fn item(key: &str, needs: &[&str], task: &str) -> String {
    let needs: Vec<String> = needs.iter().map(|need| format!("{need:?}")).collect();
    format!(
        r#"{{ "key": {key:?}, "needs": [{}], "title": "t", "files": {{ "task": {task:?} }} }}"#,
        needs.join(", ")
    )
}

pub(super) fn list(items: &[String]) -> String {
    format!("[{}]", items.join(", "))
}

pub(super) async fn problem(delivery: Delivery) -> ItemListProblem {
    let rejection = Execution::with_contract(planner_contract())
        .delivery_rejection(delivery)
        .await;
    let DeliveryRejection::InvalidItemList { path, problem } = rejection else {
        panic!("expected an invalid item list, got {rejection:?}");
    };
    assert_eq!(path, "manifest.json");
    problem
}

pub(super) async fn file_rejection(delivery: Delivery) -> DeliveryRejection {
    Execution::with_contract(planner_contract())
        .delivery_rejection(delivery)
        .await
}
