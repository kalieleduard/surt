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

#[tokio::test(flavor = "multi_thread")]
async fn a_local_agent_toml_naming_another_id_refuses() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "agent.toml", "id = \"author\"\n");

    let invalid = refused(&dir).await;

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

#[cfg(not(feature = "test-support"))]
#[tokio::test(flavor = "multi_thread")]
async fn a_fake_runtime_in_an_effective_profile_refuses_as_test_only() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "agent.toml", "[profile]\nruntime = \"fake\"\n");

    let invalid = refused(&dir).await;

    assert_eq!(
        (invalid.field.as_deref(), invalid.problem),
        (Some("profile.runtime"), Problem::TestOnlyRuntime)
    );
}
