use toml::Table;

use super::{agent, agent_ids, strings};

const READ_ONLY_GIT: [&str; 5] = [
    "Bash(git status *)",
    "Bash(git diff *)",
    "Bash(git show *)",
    "Bash(git log *)",
    "Bash(git rev-parse *)",
];

#[test]
fn every_agent_runs_on_the_shared_profile_without_turn_or_cost_limits() {
    for id in agent_ids() {
        let agent = agent(&id);
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
fn the_builders_commit_is_proven_by_git_and_the_reviewers_verdict_is_bound_to_it() {
    let declared = |id: &str, output: &str| agent(id)["outputs"][output].get("checks").cloned();
    let expected = |text: &str| {
        let table: Table = text.parse().expect("the expected checks parse");
        table.get("checks").cloned()
    };

    assert_eq!(
        declared("builder", "commit"),
        expected(r#"checks = [{ kind = "commit_evidence" }]"#)
    );
    assert_eq!(
        declared("reviewer", "feedback"),
        expected(
            r#"checks = [{ kind = "reviewed_commit" }, { kind = "no_blocking_finding", outcome = "approved" }]"#
        )
    );
    for id in ["spec", "tasks", "shipper"] {
        let agent = agent(id);
        let outputs = agent["outputs"].as_table().expect("outputs");
        assert!(
            outputs
                .values()
                .all(|output| output.get("checks").is_none()),
            "{id}"
        );
    }
}
