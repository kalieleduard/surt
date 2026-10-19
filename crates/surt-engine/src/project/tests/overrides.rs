use super::fixtures::{ProjectDir, default_config, draft_catalog, new_run};
use crate::project::{AgentAssets, Catalog, CreatedRun, LocalFiles};

const MODEL_AND_LIMITS: &str = r#"
[profile]
model = "fake-model-large"
effort = "max"

[profile.limits]
max_turns = 40
"#;

#[tokio::test(flavor = "multi_thread")]
async fn a_local_agent_toml_is_merged_over_the_built_in_and_frozen() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "agent.toml", MODEL_AND_LIMITS);

    dir.project(draft_catalog(), default_config())
        .await
        .create_run(new_run("draft"))
        .await
        .expect("the run is created");

    let frozen = dir.frozen_agent("writer").await;
    assert_eq!(
        frozen["profile"]["model"].as_str(),
        Some("fake-model-large")
    );
    assert_eq!(frozen["profile"]["effort"].as_str(), Some("max"));
    assert_eq!(frozen["profile"]["runtime"].as_str(), Some("fake"));
    assert_eq!(
        frozen["profile"]["limits"]["max_turns"].as_integer(),
        Some(40)
    );
    assert_eq!(
        frozen["outputs"]["notes"]["json_schema"].as_str(),
        Some("schemas/notes.schema.json")
    );
}

async fn created(dir: &ProjectDir) -> CreatedRun {
    dir.project(draft_catalog(), default_config())
        .await
        .create_run(new_run("draft"))
        .await
        .expect("the run is created")
}

#[tokio::test(flavor = "multi_thread")]
async fn create_run_reports_the_agents_that_used_local_files() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "agent.toml", MODEL_AND_LIMITS);

    let created = created(&dir).await;

    assert_eq!(
        created.local_agents,
        [LocalFiles {
            agent: "writer".to_owned(),
            files: vec!["agent.toml".to_owned()],
        }]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn without_local_files_create_run_reports_no_local_agents() {
    let dir = ProjectDir::new();

    let created = created(&dir).await;

    assert_eq!(created.local_agents, []);
}

const REVIEW_FLOW: &str = r#"
schema = 1
name = "flow"

[steps.review]
kind = "agent"
agent = "reviewer"
"#;

const REVIEWER: &str = r#"
schema = 1
id = "reviewer"

[outputs.feedback]
format = "text"

[profile]
runtime = "claude-code"
model = "claude-sonnet-5-5"
effort = "high"

[profile.claude]
auth = "subscription"
permission_mode = "dontAsk"
tools = ["Read", "Grep", "Bash"]
allowed_tools = ["Read", "Grep"]
disallowed_tools = ["mcp__*"]
"#;

#[tokio::test(flavor = "multi_thread")]
async fn a_local_array_replaces_the_built_in_array_and_tables_merge_by_key() {
    let dir = ProjectDir::new();
    dir.write_local(
        "reviewer",
        "agent.toml",
        "[profile.claude]\nallowed_tools = [\"Bash(npm test *)\"]\n",
    );
    let catalog = Catalog::new().workflow("flow", REVIEW_FLOW).agent(
        "reviewer",
        AgentAssets {
            toml: REVIEWER,
            instructions: "Review.\n",
            schemas: &[],
        },
    );

    dir.project(catalog, default_config())
        .await
        .create_run(new_run("flow"))
        .await
        .expect("the run is created");

    let claude = &dir.frozen_agent("reviewer").await["profile"]["claude"];
    let strings = |key: &str| -> Vec<String> {
        claude[key]
            .as_array()
            .expect("an array")
            .iter()
            .map(|tool| tool.as_str().expect("a string").to_owned())
            .collect()
    };
    assert_eq!(strings("allowed_tools"), ["Bash(npm test *)"]);
    assert_eq!(strings("tools"), ["Read", "Grep", "Bash"]);
    assert_eq!(claude["auth"].as_str(), Some("subscription"));
}

#[tokio::test(flavor = "multi_thread")]
async fn crlf_and_lf_versions_of_a_local_agent_toml_freeze_to_the_same_identity() {
    let mut identities = Vec::new();
    for text in [
        MODEL_AND_LIMITS.to_owned(),
        MODEL_AND_LIMITS.replace('\n', "\r\n"),
    ] {
        let dir = ProjectDir::new();
        dir.write_local("writer", "agent.toml", &text);
        created(&dir).await;
        let identity: Vec<(String,)> = dir
            .rows("SELECT sha256 FROM snapshot WHERE kind = 'agent' AND name = 'writer'")
            .await;
        identities.push(identity);
    }

    assert_eq!(identities[0], identities[1]);
}
