use toml::Table;

use super::{agent, file, strings};

pub(super) fn feature_steps() -> Table {
    let workflow: Table = file("workflows/feature.toml")
        .expect("the feature workflow")
        .parse()
        .expect("the workflow TOML parses");
    workflow["steps"].as_table().expect("steps").clone()
}

#[test]
fn the_spec_loop_starts_after_discovery_and_only_asks_for_approval() {
    let steps = feature_steps();
    let approve = &steps["approve-spec"];

    assert_eq!(
        strings(&steps["spec-loop"]["body"]),
        ["spec", "approve-spec"]
    );
    assert_eq!(strings(&steps["spec-loop"]["needs"]), ["discovery-loop"]);
    assert_eq!(approve["kind"].as_str(), Some("approval"));
    assert_eq!(strings(&approve["needs"]), ["spec"]);
    assert!(approve.get("when").is_none());
    assert!(!steps.contains_key("answer-spec"));
}

#[test]
fn the_spec_agent_writes_from_the_brief_without_asking_and_continues_its_conversation() {
    let spec = agent("spec");
    let inputs = &spec["inputs"];

    assert!(spec.get("outcomes").is_none());
    assert_eq!(
        spec["outputs"]
            .as_table()
            .expect("outputs")
            .keys()
            .collect::<Vec<_>>(),
        ["spec"]
    );
    assert_eq!(inputs["brief"]["format"].as_str(), Some("markdown"));
    assert!(inputs["brief"].get("optional").is_none());
    assert_eq!(
        inputs["discovery-answers"]["optional"].as_bool(),
        Some(true)
    );
    assert!(inputs.get("spec-answers").is_none());
    assert_eq!(spec["profile"]["conversation"].as_str(), Some("loop"));
}

#[test]
fn the_spec_instructions_record_decisions_instead_of_asking_questions() {
    let instructions = file("agents/spec/instructions.md").expect("the instructions");

    for named in [
        "`brief`",
        "`discovery-answers`",
        "outcome `done`",
        "**Decisions**",
    ] {
        assert!(instructions.contains(named), "{named}");
    }
    for absent in ["`questions.json`", "outcome `questions`", "Open questions"] {
        assert!(!instructions.contains(absent), "{absent}");
    }
}
