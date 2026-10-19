mod roles;
#[cfg(not(feature = "test-support"))]
mod runtime;
mod schemas;

use std::collections::BTreeSet;
use std::fs;

use surt_engine::project::check_workflow;
use tempfile::TempDir;
use toml::{Table, Value};

use super::{FEATURE_TEMPLATE, TEMPLATES, Template, named};

const FEATURE_WORKFLOW: &str = "feature";
const REQUEST_INPUT: &str = "request";
const BASE_INPUT: &str = "base";

fn feature() -> Template {
    named(FEATURE_TEMPLATE).expect("the feature template")
}

pub(super) fn file(rel_path: &str) -> Option<&'static str> {
    feature()
        .files()
        .into_iter()
        .find(|(path, _)| path == rel_path)
        .map(|(_, text)| text)
}

pub(super) fn agent(id: &str) -> Table {
    file(&format!("agents/{id}/agent.toml"))
        .unwrap_or_else(|| panic!("no agent {id} in the template"))
        .parse()
        .expect("the agent TOML parses")
}

pub(super) fn agent_ids() -> BTreeSet<String> {
    feature()
        .files()
        .into_iter()
        .filter_map(|(path, _)| {
            let id = path.strip_prefix("agents/")?.strip_suffix("/agent.toml")?;
            Some(id.to_owned())
        })
        .collect()
}

pub(super) fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .map(|item| item.as_str().expect("a string"))
        .collect()
}

pub(super) fn written(files: &[(String, String)]) -> TempDir {
    let project = TempDir::new().expect("a temporary directory");
    for (rel_path, text) in files {
        let path = project.path().join(".surt").join(rel_path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        fs::write(path, text).expect("the template file");
    }
    project
}

pub(super) fn owned_files() -> Vec<(String, String)> {
    feature()
        .files()
        .into_iter()
        .map(|(path, text)| (path, text.to_owned()))
        .collect()
}

#[test]
fn the_written_feature_template_passes_the_engines_static_validation() {
    let project = written(&owned_files());

    let checked = check_workflow(project.path(), FEATURE_WORKFLOW);

    assert!(checked.is_ok(), "{checked:?}");
}

#[test]
fn feature_is_the_only_template() {
    assert_eq!(TEMPLATES, ["feature"]);
    assert!(named("bugfix").is_none());
}

#[test]
fn the_feature_workflow_ends_with_the_fast_forward_into_the_base_after_the_shipper() {
    let feature: Table = file("workflows/feature.toml")
        .expect("the feature workflow")
        .parse()
        .expect("the workflow TOML parses");
    let merge = &feature["steps"]["merge"];

    assert_eq!(merge["kind"].as_str(), Some("operation"));
    assert_eq!(merge["run"].as_str(), Some("git.merge_into_base"));
    assert_eq!(strings(&merge["needs"]), ["ship"]);
    assert_eq!(
        feature["steps"]
            .as_table()
            .and_then(|steps| steps.keys().next_back())
            .map(String::as_str),
        Some("merge")
    );
}

#[test]
fn the_template_holds_exactly_five_agents_each_with_its_instructions() {
    let ids = agent_ids();

    assert_eq!(
        ids,
        ["builder", "reviewer", "shipper", "spec", "tasks"]
            .map(str::to_owned)
            .into()
    );
    for id in ids {
        assert!(
            file(&format!("agents/{id}/instructions.md")).is_some(),
            "{id}"
        );
    }
}

#[test]
fn the_feature_workflow_takes_a_text_request_and_a_git_base_defaulting_to_head() {
    let workflow: Table = file("workflows/feature.toml")
        .expect("the feature workflow")
        .parse()
        .expect("the workflow TOML parses");
    let inputs = &workflow["inputs"];

    assert_eq!(inputs[REQUEST_INPUT]["format"].as_str(), Some("text"));
    assert_eq!(inputs[BASE_INPUT]["format"].as_str(), Some("git_ref"));
    assert_eq!(inputs[BASE_INPUT]["default"].as_str(), Some("HEAD"));
    assert!(inputs[REQUEST_INPUT].get("description").is_some());
    assert!(inputs[BASE_INPUT].get("description").is_some());
    assert_eq!(workflow["subject"]["from"].as_str(), Some(REQUEST_INPUT));
    assert_eq!(
        feature().next_command(),
        "surt run feature --input request=\"<what you want to build>\""
    );
}
