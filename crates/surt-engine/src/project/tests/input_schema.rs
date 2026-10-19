use super::fixtures::{
    ProjectDir, default_config, draft_catalog, in_step, new_run, refusal, workflow_refusal,
};
use super::per_item::catalog;
use crate::project::{DefinitionFile, InputFormat, InvalidDefinition, Problem};

fn at_field(field: &str, problem: Problem) -> InvalidDefinition {
    InvalidDefinition {
        file: DefinitionFile::Workflow("flow".to_owned()),
        field: Some(field.to_owned()),
        step: None,
        problem,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_default_is_refused_on_an_input_that_is_not_a_git_ref() {
    let invalid = workflow_refusal(
        "schema = 1\nname = \"flow\"\n\n[inputs.request]\nformat = \"text\"\ndefault = \"add login\"\n\n[steps.write]\nkind = \"agent\"\nagent = \"writer\"\n",
    )
    .await;

    assert_eq!(
        invalid,
        at_field(
            "inputs.request.default",
            Problem::MisplacedInputDefault {
                input: "request".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_subject_must_come_from_a_declared_text_or_markdown_input() {
    let undeclared = workflow_refusal(
        "schema = 1\nname = \"flow\"\n\n[inputs.request]\nformat = \"text\"\n\n[subject]\nfrom = \"title\"\n\n[steps.write]\nkind = \"agent\"\nagent = \"writer\"\n",
    )
    .await;
    let json = workflow_refusal(
        "schema = 1\nname = \"flow\"\n\n[inputs.request]\nformat = \"json\"\n\n[subject]\nfrom = \"request\"\n\n[steps.write]\nkind = \"agent\"\nagent = \"writer\"\n",
    )
    .await;

    assert_eq!(
        undeclared,
        at_field(
            "subject.from",
            Problem::UnknownSubjectInput {
                input: "title".to_owned()
            }
        )
    );
    assert_eq!(
        json,
        at_field(
            "subject.from",
            Problem::SubjectInputNotText {
                input: "request".to_owned(),
                format: InputFormat::Json,
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_field_of_an_input_is_refused() {
    let invalid = workflow_refusal(
        "schema = 1\nname = \"flow\"\n\n[inputs.request]\nformat = \"text\"\nrequired = true\n\n[steps.write]\nkind = \"agent\"\nagent = \"writer\"\n",
    )
    .await;

    assert!(
        matches!(&invalid.problem, Problem::Syntax { detail, .. } if detail.contains("required")),
        "{invalid:?}"
    );
    assert_eq!(invalid.field.as_deref(), Some("inputs.request.required"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_per_item_child_that_declares_inputs_is_refused() {
    let invalid = refusal(catalog(
        "schema = 1\nname = \"flow\"\n\n[inputs.request]\nformat = \"text\"\n\n[steps.plan]\nkind = \"agent\"\nagent = \"planner\"\n\n[steps.implement]\nkind = \"per_item\"\nitems = \"plan\"\nworkflow = \"child\"\npass = [\"request\"]\nneeds = [\"plan\"]\n",
        "schema = 1\nname = \"child\"\n\n[inputs.request]\nformat = \"text\"\n\n[steps.write]\nkind = \"agent\"\nagent = \"writer\"\n",
    ))
    .await;

    assert_eq!(
        invalid,
        InvalidDefinition {
            file: DefinitionFile::Workflow("child".to_owned()),
            field: Some("inputs".to_owned()),
            step: None,
            problem: Problem::InputsInChildWorkflow,
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_git_operation_needs_its_base_from_a_git_ref_input() {
    let text_base = workflow_refusal(
        "schema = 1\nname = \"flow\"\n\n[inputs.request]\nformat = \"text\"\n\n[inputs.base]\nformat = \"text\"\n\n[steps.prepare]\nkind = \"operation\"\nrun = \"git.create_run_branch\"\n",
    )
    .await;
    let undeclared = workflow_refusal(
        "schema = 1\nname = \"flow\"\n\n[inputs.request]\nformat = \"text\"\n\n[steps.prepare]\nkind = \"operation\"\nrun = \"git.create_run_branch\"\n",
    )
    .await;

    assert_eq!(
        text_base,
        in_step(
            "prepare",
            Problem::GitInputNotGitRef {
                input: "base".to_owned()
            }
        )
    );
    assert_eq!(
        undeclared,
        in_step(
            "prepare",
            Problem::MissingInput {
                input: "base".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn inputs_and_the_subject_are_part_of_the_frozen_canonical_workflow() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    project
        .create_run(new_run("draft"))
        .await
        .expect("the run is created");

    let frozen: Vec<(String,)> = dir
        .rows("SELECT CAST(b.bytes AS TEXT) FROM snapshot s JOIN snapshot_file f ON f.snapshot_sha256 = s.sha256 JOIN blob b ON b.sha256 = f.blob_sha256 WHERE s.kind = 'workflow'")
        .await;
    let canonical: toml::Table = toml::from_str(&frozen[0].0).expect("the frozen workflow is TOML");

    assert_eq!(
        canonical["inputs"]["base"]["format"].as_str(),
        Some("git_ref")
    );
    assert_eq!(
        canonical["inputs"]["base"]["default"].as_str(),
        Some("HEAD")
    );
    assert_eq!(
        canonical["inputs"]["request"]["format"].as_str(),
        Some("text")
    );
    assert_eq!(canonical["subject"]["from"].as_str(), Some("request"));
}
