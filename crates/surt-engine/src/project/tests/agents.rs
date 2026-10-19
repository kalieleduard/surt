use super::fixtures::{AgentAssets, Catalog};
use super::fixtures::{ProjectDir, agent, default_config, new_run, refusal};
use crate::project::{DefinitionFile, InvalidDefinition, Problem};

const FLOW: &str = r#"
schema = 1
name = "flow"

[steps.work]
kind = "agent"
agent = "worker"
"#;

const FAKE_PROFILE: &str = r#"
[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

fn catalog(assets: AgentAssets) -> Catalog {
    Catalog::new()
        .workflow("flow", FLOW)
        .agent("worker", assets)
}

fn in_agent(field: Option<&str>, problem: Problem) -> InvalidDefinition {
    InvalidDefinition {
        file: DefinitionFile::Agent("worker".to_owned()),
        field: field.map(str::to_owned),
        step: None,
        problem,
    }
}

async fn agent_refusal(toml: &'static str) -> InvalidDefinition {
    refusal(catalog(agent(toml))).await
}

fn with_fake_profile(head: &str) -> &'static str {
    format!("{head}{FAKE_PROFILE}").leak()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_complete_claude_code_contract_is_accepted() {
    let dir = ProjectDir::new();
    let toml = r#"
schema = 1
id = "worker"
outcomes = ["approved", "changes_requested"]

[inputs.request]
format = "text"

[outputs.feedback]
format = "json"
json_schema = "schemas/feedback.schema.json"

[profile]
runtime = "claude-code"
model = "claude-sonnet-5-5"
effort = "high"

[profile.limits]
max_turns = 100
max_budget_usd = 5.0

[profile.claude]
auth = "subscription"
permission_mode = "dontAsk"
tools = ["Read", "Write"]
allowed_tools = ["Read"]
disallowed_tools = ["mcp__*"]
"#;
    let assets = AgentAssets {
        toml,
        instructions: "Review.\n",
        schemas: &[("schemas/feedback.schema.json", "{}")],
    };
    let project = dir.project(catalog(assets), default_config()).await;

    let created = project.create_run(new_run("flow")).await;

    assert!(created.is_ok(), "{created:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_field_at_any_level_of_an_agent_is_refused() {
    let invalid = agent_refusal(with_fake_profile(
        "schema = 1\nid = \"worker\"\n\n[outputs.notes]\nformat = \"text\"\nrequired = true\n",
    ))
    .await;

    assert!(
        matches!(&invalid.problem, Problem::Syntax { line: Some(6), detail } if detail.contains("required")),
        "{invalid:?}"
    );
    assert_eq!(invalid.file, DefinitionFile::Agent("worker".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_with_an_unknown_schema_or_a_foreign_id_is_refused() {
    let schema = agent_refusal(with_fake_profile("schema = 2\nid = \"worker\"\n")).await;
    let id = agent_refusal(with_fake_profile("schema = 1\nid = \"builder\"\n")).await;

    assert_eq!(
        schema,
        in_agent(Some("schema"), Problem::UnsupportedSchema { found: 2 })
    );
    assert_eq!(
        id,
        in_agent(
            Some("id"),
            Problem::NameMismatch {
                found: "builder".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn every_referenced_schema_must_be_an_asset_of_the_agent() {
    let invalid = agent_refusal(with_fake_profile(
        "schema = 1\nid = \"worker\"\n\n[outputs.notes]\nformat = \"json\"\njson_schema = \"schemas/notes.schema.json\"\n",
    ))
    .await;

    assert_eq!(
        invalid,
        in_agent(
            None,
            Problem::MissingAsset {
                path: "schemas/notes.schema.json".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn json_schema_only_applies_to_json_and_item_lists() {
    let invalid = agent_refusal(with_fake_profile(
        "schema = 1\nid = \"worker\"\n\n[inputs.spec]\nformat = \"markdown\"\njson_schema = \"schemas/spec.schema.json\"\n",
    ))
    .await;

    assert_eq!(
        invalid,
        in_agent(
            None,
            Problem::MisplacedJsonSchema {
                name: "spec".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn item_files_only_belong_to_item_lists_and_hold_plain_formats() {
    let misplaced = agent_refusal(with_fake_profile(
        "schema = 1\nid = \"worker\"\n\n[outputs.plan]\nformat = \"json\"\n\n[outputs.plan.item_files.task]\nformat = \"markdown\"\n",
    ))
    .await;
    let nested = agent_refusal(with_fake_profile(
        "schema = 1\nid = \"worker\"\n\n[outputs.plan]\nformat = \"item_list\"\n\n[outputs.plan.item_files.task]\nformat = \"item_list\"\n",
    ))
    .await;

    assert_eq!(
        misplaced,
        in_agent(
            None,
            Problem::MisplacedItemFiles {
                name: "plan".to_owned()
            }
        )
    );
    assert_eq!(
        nested,
        in_agent(
            None,
            Problem::UnsupportedItemFileFormat {
                name: "task".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_claude_code_profile_requires_its_claude_table() {
    let invalid = agent_refusal(
        "schema = 1\nid = \"worker\"\n\n[profile]\nruntime = \"claude-code\"\nmodel = \"claude-sonnet-5-5\"\neffort = \"high\"\n",
    )
    .await;

    assert_eq!(
        invalid,
        in_agent(Some("profile.claude"), Problem::MissingClaudeProfile)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn api_key_env_requires_api_key_auth() {
    let invalid = agent_refusal(
        "schema = 1\nid = \"worker\"\n\n[profile]\nruntime = \"claude-code\"\nmodel = \"claude-sonnet-5-5\"\neffort = \"high\"\n\n[profile.claude]\nauth = \"subscription\"\npermission_mode = \"dontAsk\"\ntools = []\nallowed_tools = []\ndisallowed_tools = []\napi_key_env = \"MY_KEY\"\n",
    )
    .await;

    assert_eq!(
        invalid,
        in_agent(
            Some("profile.claude.api_key_env"),
            Problem::MisplacedApiKeyEnv
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_budget_limit_must_be_positive_and_finite() {
    let invalid = agent_refusal(
        "schema = 1\nid = \"worker\"\n\n[profile]\nruntime = \"fake\"\nmodel = \"fake-model\"\neffort = \"high\"\n\n[profile.limits]\nmax_budget_usd = -1.0\n",
    )
    .await;

    assert_eq!(
        invalid,
        in_agent(
            Some("profile.limits.max_budget_usd"),
            Problem::InvalidBudget
        )
    );
}
