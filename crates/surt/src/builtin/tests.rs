use std::collections::BTreeSet;

use surt_engine::project::{CHECK_REPORT_SCHEMA, DeliveryCheck};
use toml::{Table, Value};

use super::{AGENTS, BASE_ARTIFACT, FEATURE_WORKFLOW, REQUEST_ARTIFACT, catalog};

const READ_ONLY_GIT: [&str; 5] = [
    "Bash(git status *)",
    "Bash(git diff *)",
    "Bash(git show *)",
    "Bash(git log *)",
    "Bash(git rev-parse *)",
];

fn agent(id: &str) -> Table {
    let (_, assets) = AGENTS
        .iter()
        .find(|(candidate, _)| *candidate == id)
        .unwrap_or_else(|| panic!("no built-in agent {id}"));
    assets.toml.parse().expect("the agent TOML parses")
}

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .map(|item| item.as_str().expect("a string"))
        .collect()
}

#[test]
fn the_built_in_feature_workflow_and_its_agents_pass_static_validation() {
    assert_eq!(
        catalog().check(FEATURE_WORKFLOW, &[REQUEST_ARTIFACT, BASE_ARTIFACT]),
        Ok(())
    );
    assert_eq!([REQUEST_ARTIFACT, BASE_ARTIFACT], ["request", "base"]);
}

#[test]
fn the_catalog_holds_exactly_the_five_built_in_agents() {
    let ids: BTreeSet<&str> = AGENTS.iter().map(|(id, _)| *id).collect();

    assert_eq!(
        ids,
        BTreeSet::from(["builder", "reviewer", "shipper", "spec", "tasks"])
    );
}

#[test]
fn every_agent_runs_on_the_shared_profile_without_turn_or_cost_limits() {
    for (id, _) in AGENTS {
        let agent = agent(id);
        let profile = agent["profile"].as_table().expect("a profile");
        let claude = profile["claude"].as_table().expect("a claude profile");

        assert_eq!(profile["runtime"].as_str(), Some("claude-code"), "{id}");
        assert_eq!(profile["model"].as_str(), Some("claude-sonnet-5-5"), "{id}");
        assert_eq!(profile["effort"].as_str(), Some("high"), "{id}");
        assert!(!profile.contains_key("limits"), "{id}");
        assert_eq!(claude["auth"].as_str(), Some("subscription"), "{id}");
        assert_eq!(claude["permission_mode"].as_str(), Some("dontAsk"), "{id}");
        assert_eq!(strings(&claude["disallowed_tools"]), ["mcp__*"], "{id}");
    }
}

#[test]
fn each_role_gets_the_tools_and_approvals_of_its_row_in_the_agent_contract() {
    let reviewer_approvals = [&["Read", "Glob", "Grep"][..], &READ_ONLY_GIT[..]].concat();
    let expected: [(&str, &[&str], &[&str]); 5] = [
        (
            "spec",
            &["Read", "Glob", "Grep", "Write"],
            &["Read", "Glob", "Grep"],
        ),
        (
            "tasks",
            &["Read", "Glob", "Grep", "Write"],
            &["Read", "Glob", "Grep"],
        ),
        (
            "reviewer",
            &["Read", "Glob", "Grep", "Write", "Bash"],
            &reviewer_approvals,
        ),
        (
            "shipper",
            &["Read", "Glob", "Grep", "Write", "Bash"],
            &reviewer_approvals,
        ),
        (
            "builder",
            &["Read", "Glob", "Grep", "Write", "Edit", "Bash"],
            &["Read", "Glob", "Grep", "Bash"],
        ),
    ];

    for (id, tools, approvals) in expected {
        let agent = agent(id);
        let claude = &agent["profile"]["claude"];

        assert_eq!(strings(&claude["tools"]), tools, "{id}");
        assert_eq!(strings(&claude["allowed_tools"]), approvals, "{id}");
    }
}

#[test]
fn only_the_builders_commit_must_be_proven_by_git() {
    let catalog = catalog();

    for (id, _) in AGENTS {
        let expected: &[(String, DeliveryCheck)] = if id == "builder" {
            &[("commit".to_owned(), DeliveryCheck::CommitEvidence)]
        } else {
            &[]
        };
        assert_eq!(catalog.delivery_checks(id), expected, "{id}");
    }
}

#[test]
fn agents_reading_a_check_report_validate_it_with_the_engines_schema() {
    let (path, text) = CHECK_REPORT_SCHEMA;

    for id in ["builder", "reviewer"] {
        let agent = agent(id);
        let (_, assets) = AGENTS
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .expect("the agent");

        assert_eq!(
            agent["inputs"]["check-report"]["json_schema"].as_str(),
            Some(path),
            "{id}"
        );
        assert!(assets.schemas.contains(&(path, text)), "{id}");
    }
}

#[test]
fn every_embedded_schema_is_a_draft_07_json_schema() {
    for (id, assets) in AGENTS {
        for (path, text) in assets.schemas {
            let schema: serde_json::Value =
                serde_json::from_str(text).unwrap_or_else(|error| panic!("{id} {path}: {error}"));

            assert_eq!(
                schema["$schema"].as_str(),
                Some("http://json-schema.org/draft-07/schema#"),
                "{id} {path}"
            );
        }
    }
}
