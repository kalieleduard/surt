use super::fixtures::{ProjectDir, WRITER_AGENT, default_config, draft_catalog, invalid_file};
use crate::project::{InvalidDefinition, Problem};

async fn refused(agent_toml: &str) -> InvalidDefinition {
    let dir = ProjectDir::new();
    dir.project(draft_catalog(), default_config()).await;
    dir.write_agent_file("writer", "agent.toml", agent_toml);
    invalid_file(&dir, "draft").await
}

fn writer_with(from: &str, to: &str) -> String {
    assert!(WRITER_AGENT.contains(from), "{from}");
    WRITER_AGENT.replace(from, to)
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_field_refuses_with_its_line_and_field_path() {
    let invalid = refused(&format!(
        "{WRITER_AGENT}\n[profile.limits]\nmax_turnz = 3\n"
    ))
    .await;

    assert_eq!(invalid.file.path(), ".surt/agents/writer/agent.toml");
    assert_eq!(invalid.field.as_deref(), Some("profile.limits.max_turnz"));
    assert!(
        matches!(&invalid.problem, Problem::Syntax { line: Some(_), detail } if detail.contains("unknown field `max_turnz`")),
        "{invalid:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failing_profile_check_refuses_with_its_field_path() {
    let invalid = refused(&format!(
        "{WRITER_AGENT}\n[profile.limits]\nmax_budget_usd = -1.0\n"
    ))
    .await;

    assert_eq!(
        (invalid.field.as_deref(), invalid.problem),
        (
            Some("profile.limits.max_budget_usd"),
            Problem::InvalidBudget
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_toml_that_is_not_toml_refuses_with_its_line() {
    let invalid = refused("schema = 1\nid = \n").await;

    assert_eq!(invalid.field.as_deref(), Some("id"));
    assert!(
        matches!(invalid.problem, Problem::Syntax { line: Some(2), .. }),
        "{invalid:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_toml_naming_another_id_refuses_at_id() {
    let invalid = refused(&writer_with("id = \"writer\"", "id = \"author\"")).await;

    assert_eq!(
        (invalid.field.as_deref(), invalid.problem),
        (
            Some("id"),
            Problem::NameMismatch {
                found: "author".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_toml_missing_a_required_field_refuses() {
    let invalid = refused(&writer_with("effort = \"high\"\n", "")).await;

    assert!(
        matches!(&invalid.problem, Problem::Syntax { detail, .. } if detail.contains("effort")),
        "{invalid:?}"
    );
}
