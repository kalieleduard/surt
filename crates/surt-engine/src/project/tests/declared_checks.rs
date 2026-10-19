use super::fixtures::{ProjectDir, agent, default_config, new_run, refusal};
use super::override_workflows::{UNTIL_DONE, WRITER_FILE, refused};
use crate::project::{Catalog, DefinitionFile, InvalidDefinition, InvalidOverride, Problem};

const FLOW: &str = r#"
schema = 1
name = "flow"

[steps.write]
kind = "agent"
agent = "writer"
"#;

const HEAD: &str = r#"
schema = 1
id = "writer"
outcomes = ["done", "approved"]

[inputs.request]
format = "text"
"#;

const FAKE_PROFILE: &str = r#"
[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const NOTES_CHECKED: &str = r#"
[outputs.notes]
format = "json"
checks = [{ kind = "commit_evidence" }, { kind = "no_blocking_finding", outcome = "approved" }]
"#;

fn writer_with(outputs: &str) -> &'static str {
    format!("{HEAD}{outputs}{FAKE_PROFILE}").leak()
}

fn flow(outputs: &str) -> Catalog {
    Catalog::new()
        .workflow("flow", FLOW)
        .agent("writer", agent(writer_with(outputs)))
}

fn in_writer(problem: Problem) -> InvalidDefinition {
    InvalidDefinition {
        file: DefinitionFile::Agent("writer".to_owned()),
        step: None,
        problem,
    }
}

async fn syntax_refusal(checks: &str) -> String {
    let outputs = format!("\n[outputs.notes]\nformat = \"json\"\nchecks = [{checks}]\n");
    match refusal(flow(&outputs)).await.problem {
        Problem::Syntax { detail, .. } => detail,
        other => panic!("expected a syntax problem, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_check_on_an_output_that_is_not_json_refuses_create_run_with_no_row_written() {
    let invalid = refusal(flow(
        "\n[outputs.draft]\nformat = \"markdown\"\nchecks = [{ kind = \"reviewed_commit\" }]\n",
    ))
    .await;

    assert_eq!(
        invalid,
        in_writer(Problem::MissingCheckedOutput {
            output: "draft".to_owned()
        })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_blocking_finding_on_an_outcome_the_agent_does_not_declare_refuses_create_run() {
    let invalid = refusal(flow(
        "\n[outputs.notes]\nformat = \"json\"\nchecks = [{ kind = \"no_blocking_finding\", outcome = \"accepted\" }]\n",
    ))
    .await;

    assert_eq!(
        invalid,
        in_writer(Problem::MissingCheckedOutcome {
            outcome: "accepted".to_owned()
        })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_check_of_an_unknown_kind_or_with_a_field_its_kind_does_not_take_is_refused() {
    let unknown_kind = syntax_refusal("{ kind = \"signed_commit\" }").await;
    let stray_outcome =
        syntax_refusal("{ kind = \"commit_evidence\", outcome = \"approved\" }").await;
    let stray_field =
        syntax_refusal("{ kind = \"no_blocking_finding\", outcome = \"approved\", severity = 1 }")
            .await;
    let missing_outcome = syntax_refusal("{ kind = \"no_blocking_finding\" }").await;

    assert!(unknown_kind.contains("signed_commit"), "{unknown_kind}");
    assert!(stray_outcome.contains("outcome"), "{stray_outcome}");
    assert!(stray_field.contains("severity"), "{stray_field}");
    assert!(
        missing_outcome.contains("missing field `outcome`"),
        "{missing_outcome}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn declared_checks_are_frozen_in_the_canonical_agent_toml() {
    let dir = ProjectDir::new();
    dir.project(flow(NOTES_CHECKED), default_config())
        .await
        .create_run(new_run("flow"))
        .await
        .expect("the run is created");

    let frozen = dir.frozen_agent("writer").await;

    let declared: toml::Table = toml::from_str(NOTES_CHECKED).expect("the checks are TOML");
    assert_eq!(
        frozen["outputs"]["notes"]["checks"],
        declared["outputs"]["notes"]["checks"]
    );
}

fn checked_drafting() -> Catalog {
    Catalog::new()
        .workflow("drafting", UNTIL_DONE)
        .agent("writer", agent(writer_with(NOTES_CHECKED)))
}

#[tokio::test(flavor = "multi_thread")]
async fn an_override_that_stops_delivering_a_checked_output_as_json_is_refused_at_its_format() {
    let dir = ProjectDir::new();
    dir.write_local(
        "writer",
        "agent.toml",
        "[outputs.notes]\nformat = \"text\"\n",
    );

    let invalid = refused(&dir, checked_drafting()).await;

    assert_eq!(
        invalid,
        InvalidOverride {
            file: WRITER_FILE.to_owned(),
            field: Some("outputs.notes.format".to_owned()),
            needed_by: None,
            problem: Problem::MissingCheckedOutput {
                output: "notes".to_owned()
            },
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_override_removing_the_outcome_a_check_names_is_refused_at_its_outcomes() {
    let dir = ProjectDir::new();
    dir.write_local(
        "writer",
        "agent.toml",
        "outcomes = [\"done\", \"blocked\"]\n",
    );

    let invalid = refused(&dir, checked_drafting()).await;

    assert_eq!(
        invalid,
        InvalidOverride {
            file: WRITER_FILE.to_owned(),
            field: Some("outcomes".to_owned()),
            needed_by: None,
            problem: Problem::MissingCheckedOutcome {
                outcome: "approved".to_owned()
            },
        }
    );
}
