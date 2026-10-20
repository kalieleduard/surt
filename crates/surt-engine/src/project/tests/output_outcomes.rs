use super::fixtures::Catalog;
use super::fixtures::{ProjectDir, agent, default_config, new_run, refusal};
use crate::project::{DefinitionFile, InvalidDefinition, Problem};

const FLOW: &str = r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.write]
kind = "agent"
agent = "writer"
"#;

const HEAD: &str = r#"
schema = 1
id = "writer"
outcomes = ["done", "questions"]

[inputs.request]
format = "text"
"#;

const FAKE_PROFILE: &str = r#"
[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const SPEC_ON_DONE: &str = r#"
[outputs.spec]
format = "markdown"
outcomes = ["done"]

[outputs.notes]
format = "text"
"#;

fn flow(outputs: &str) -> Catalog {
    let toml: &'static str = format!("{HEAD}{outputs}{FAKE_PROFILE}").leak();
    Catalog::new()
        .workflow("flow", FLOW)
        .agent("writer", agent(toml))
}

fn spec_outcomes(outcomes: &str) -> Catalog {
    flow(&format!(
        "\n[outputs.spec]\nformat = \"markdown\"\noutcomes = {outcomes}\n"
    ))
}

fn in_writer(field: &str, problem: Problem) -> InvalidDefinition {
    InvalidDefinition {
        file: DefinitionFile::Agent("writer".to_owned()),
        field: Some(field.to_owned()),
        step: None,
        problem,
    }
}

async fn frozen_agent_sha(dir: &ProjectDir, catalog: Catalog) -> Vec<String> {
    dir.project(catalog, default_config())
        .await
        .create_run(new_run("flow"))
        .await
        .expect("the run is created");
    let rows: Vec<(String,)> = dir
        .rows("SELECT sha256 FROM snapshot WHERE kind = 'agent' AND name = 'writer'")
        .await;
    rows.into_iter().map(|(sha,)| sha).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_output_naming_no_outcome_is_refused_at_its_field() {
    let invalid = refusal(spec_outcomes("[]")).await;

    assert_eq!(
        invalid,
        in_writer(
            "outputs.spec.outcomes",
            Problem::EmptyOutputOutcomes {
                output: "spec".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_output_naming_an_outcome_the_agent_does_not_declare_is_refused() {
    let invalid = refusal(spec_outcomes("[\"done\", \"approved\"]")).await;

    assert_eq!(
        invalid,
        in_writer(
            "outputs.spec.outcomes",
            Problem::UnknownOutputOutcome {
                output: "spec".to_owned(),
                outcome: "approved".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_output_naming_an_outcome_twice_is_refused() {
    let invalid = refusal(spec_outcomes("[\"done\", \"questions\", \"done\"]")).await;

    assert_eq!(
        invalid,
        in_writer(
            "outputs.spec.outcomes",
            Problem::DuplicateOutputOutcome {
                output: "spec".to_owned(),
                outcome: "done".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_output_of_an_agent_with_the_default_outcome_may_only_name_done() {
    let catalog = Catalog::new().workflow("flow", FLOW).agent(
        "writer",
        agent(
            "schema = 1\nid = \"writer\"\n\n[inputs.request]\nformat = \"text\"\n\n[outputs.spec]\nformat = \"markdown\"\noutcomes = [\"questions\"]\n\n[profile]\nruntime = \"fake\"\nmodel = \"fake-model\"\neffort = \"high\"\n",
        ),
    );

    let invalid = refusal(catalog).await;

    assert_eq!(
        invalid.problem,
        Problem::UnknownOutputOutcome {
            output: "spec".to_owned(),
            outcome: "questions".to_owned()
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_blocking_finding_on_an_outcome_its_output_is_not_delivered_on_is_refused() {
    let invalid = refusal(flow(
        "\n[outputs.feedback]\nformat = \"json\"\noutcomes = [\"done\"]\nchecks = [{ kind = \"no_blocking_finding\", outcome = \"questions\" }]\n",
    ))
    .await;

    assert_eq!(
        invalid,
        in_writer(
            "outputs.feedback.outcomes",
            Problem::UndeliveredCheckedOutput {
                output: "feedback".to_owned(),
                outcome: "questions".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn output_outcomes_are_frozen_in_the_canonical_agent_toml_and_its_identity() {
    let dir = ProjectDir::new();
    let with_outcomes = frozen_agent_sha(&dir, flow(SPEC_ON_DONE)).await;
    let frozen = dir.frozen_agent("writer").await;
    let without_outcomes = frozen_agent_sha(
        &ProjectDir::new(),
        flow(&SPEC_ON_DONE.replace("outcomes = [\"done\"]\n", "")),
    )
    .await;

    assert_eq!(
        frozen["outputs"]["spec"]["outcomes"],
        toml::Value::Array(vec![toml::Value::String("done".to_owned())])
    );
    assert!(frozen["outputs"]["notes"].get("outcomes").is_none());
    assert_ne!(with_outcomes, without_outcomes);
}
