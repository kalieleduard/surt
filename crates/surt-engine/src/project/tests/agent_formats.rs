use super::fixtures::{Catalog, agent, refusal};
use crate::project::{DefinitionFile, Problem};

const FLOW: &str = r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.ask]
kind = "agent"
agent = "asker"
"#;

const FAKE_PROFILE: &str = r#"
[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

async fn asker_refusal(declarations: &str) -> Problem {
    let toml: &'static str =
        format!("schema = 1\nid = \"asker\"\n{declarations}{FAKE_PROFILE}").leak();
    let invalid = refusal(
        Catalog::new()
            .workflow("flow", FLOW)
            .agent("asker", agent(toml)),
    )
    .await;
    assert_eq!(
        (invalid.file, invalid.step),
        (DefinitionFile::Agent("asker".to_owned()), None)
    );
    invalid.problem
}

fn misplaced_schema() -> Problem {
    Problem::MisplacedJsonSchema {
        name: "asked".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn questions_carry_no_json_schema_as_an_input_or_an_output() {
    let input = asker_refusal(
        "\n[inputs.asked]\nformat = \"questions\"\njson_schema = \"schemas/q.schema.json\"\n",
    )
    .await;
    let output = asker_refusal(
        "\n[outputs.asked]\nformat = \"questions\"\njson_schema = \"schemas/q.schema.json\"\n",
    )
    .await;

    assert_eq!((input, output), (misplaced_schema(), misplaced_schema()));
}

#[tokio::test(flavor = "multi_thread")]
async fn questions_are_no_item_file_format() {
    let problem = asker_refusal(
        "\n[outputs.plan]\nformat = \"item_list\"\n\n[outputs.plan.item_files.asked]\nformat = \"questions\"\n",
    )
    .await;

    assert_eq!(
        problem,
        Problem::UnsupportedItemFileFormat {
            name: "asked".to_owned()
        }
    );
}
