use surt_engine::project::{Error, Problem, Refusal, check_workflow};

use super::{owned_files, written};
use crate::template::FEATURE_WORKFLOW;

const BUILDER: &str = "agents/builder/agent.toml";

#[test]
fn a_release_build_refuses_an_agent_on_the_test_only_fake_runtime() {
    let files: Vec<(String, String)> = owned_files()
        .into_iter()
        .map(|(path, text)| match path.as_str() {
            BUILDER => (
                path,
                text.replace("runtime = \"claude-code\"", "runtime = \"fake\""),
            ),
            _ => (path, text),
        })
        .collect();
    let project = written(&files);

    let checked = check_workflow(project.path(), FEATURE_WORKFLOW);

    let Err(Error::Refused(Refusal::InvalidDefinition(invalid))) = checked else {
        panic!("expected an invalid definition, got {checked:?}");
    };
    assert_eq!(invalid.file.path(), ".surt/agents/builder/agent.toml");
    assert_eq!(
        (invalid.field.as_deref(), invalid.problem),
        (Some("profile.runtime"), Problem::TestOnlyRuntime)
    );
}
