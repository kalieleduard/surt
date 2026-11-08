use serde_json::json;
use surt_engine::project::{DeclaredInput, Error, InputFormat, Refusal, WorkflowDefinitionSummary};

use super::WorkflowView;

#[test]
fn a_workflow_definition_reads_with_its_run_inputs_or_the_problem_that_stops_it() {
    let feature = WorkflowDefinitionSummary {
        name: "feature".to_owned(),
        run_inputs: Ok(vec![
            DeclaredInput {
                name: "request".to_owned(),
                format: InputFormat::Text,
                optional: false,
                default: None,
                description: Some("What to build".to_owned()),
            },
            DeclaredInput {
                name: "base".to_owned(),
                format: InputFormat::GitRef,
                optional: true,
                default: Some("HEAD".to_owned()),
                description: None,
            },
        ]),
    };
    let broken = WorkflowDefinitionSummary {
        name: "broken".to_owned(),
        run_inputs: Err(Error::Refused(Refusal::UnknownWorkflow {
            name: "broken".to_owned(),
            known: Vec::new(),
        })),
    };
    let problem = broken.run_inputs.as_ref().err().map(ToString::to_string);

    let views = serde_json::to_value([WorkflowView::from(feature), WorkflowView::from(broken)])
        .expect("serializes");

    assert_eq!(
        views,
        json!([
            {
                "name": "feature",
                "runInputs": [
                    { "name": "request", "format": "text", "optional": false, "default": null, "description": "What to build" },
                    { "name": "base", "format": "git_ref", "optional": true, "default": "HEAD", "description": null }
                ],
                "problem": null
            },
            { "name": "broken", "runInputs": null, "problem": problem }
        ])
    );
}
