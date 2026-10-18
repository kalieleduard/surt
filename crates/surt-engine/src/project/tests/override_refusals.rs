use super::fixtures::{ProjectDir, draft_catalog, override_refusal};
use crate::project::{InvalidOverride, Problem};

async fn refused(dir: &ProjectDir) -> InvalidOverride {
    override_refusal(dir, draft_catalog(), "draft").await
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_field_in_a_local_agent_toml_refuses_with_its_field_path() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "agent.toml", "[profile.limits]\nmax_turnz = 3\n");

    let invalid = refused(&dir).await;

    assert_eq!(invalid.file, ".surt/agents/writer/agent.toml");
    assert_eq!(invalid.field.as_deref(), Some("profile.limits.max_turnz"));
    assert!(
        matches!(&invalid.problem, Problem::Syntax { line: None, detail } if detail.contains("unknown field `max_turnz`")),
        "{invalid:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bad_effort_in_a_local_agent_toml_refuses_with_its_field_path() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "agent.toml", "[profile]\neffort = \"huge\"\n");

    let invalid = refused(&dir).await;

    assert_eq!(invalid.field.as_deref(), Some("profile.effort"));
    assert!(
        matches!(&invalid.problem, Problem::Syntax { detail, .. } if detail.contains("huge")),
        "{invalid:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failing_profile_check_on_the_merged_agent_refuses_with_its_field_path() {
    let dir = ProjectDir::new();
    dir.write_local(
        "writer",
        "agent.toml",
        "[profile.limits]\nmax_budget_usd = -1.0\n",
    );

    let invalid = refused(&dir).await;

    assert_eq!(
        (invalid.field.as_deref(), invalid.problem),
        (
            Some("profile.limits.max_budget_usd"),
            Problem::InvalidBudget
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_agent_toml_that_is_not_toml_refuses_with_its_line() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "agent.toml", "[profile]\nmodel = \n");

    let invalid = refused(&dir).await;

    assert_eq!(invalid.field, None);
    assert!(
        matches!(invalid.problem, Problem::Syntax { line: Some(2), .. }),
        "{invalid:?}"
    );
}
