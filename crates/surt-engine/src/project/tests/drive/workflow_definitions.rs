use std::fs;

use super::fixtures::Feature;
use crate::project::{DeclaredInput, Error, InputFormat, Refusal};

fn declared(name: &str, format: InputFormat, default: Option<&str>) -> DeclaredInput {
    DeclaredInput {
        name: name.to_owned(),
        format,
        optional: default.is_some(),
        default: default.map(str::to_owned),
        description: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn each_workflow_definition_lists_the_run_inputs_it_declares_in_order() {
    let feature = Feature::review().await;

    let summaries = feature
        .project
        .workflow_definitions()
        .expect("the definitions read");

    let names: Vec<&str> = summaries
        .iter()
        .map(|summary| summary.name.as_str())
        .collect();
    assert_eq!(names, ["review"]);
    let inputs = summaries[0].run_inputs.as_ref().expect("review is valid");
    assert_eq!(
        inputs,
        &[
            declared("request", InputFormat::Text, None),
            declared("base", InputFormat::GitRef, Some("HEAD")),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_invalid_workflow_definition_is_listed_with_the_refusal_a_run_of_it_would_get() {
    let feature = Feature::review().await;
    fs::write(
        feature.dir.path().join(".surt/workflows/broken.toml"),
        "schema = 1
name = \"broken\"

[steps.write]
kind = \"agent\"
agent = \"nobody\"
",
    )
    .expect("the broken definition is written");

    let summaries = feature
        .project
        .workflow_definitions()
        .expect("the definitions read");

    let names: Vec<&str> = summaries
        .iter()
        .map(|summary| summary.name.as_str())
        .collect();
    assert_eq!(names, ["broken", "review"]);
    assert!(
        matches!(
            summaries[0].run_inputs,
            Err(Error::Refused(Refusal::InvalidDefinition(_)))
        ),
        "{:?}",
        summaries[0].run_inputs
    );
    assert!(summaries[1].run_inputs.is_ok());
}
