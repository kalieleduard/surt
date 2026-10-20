use surt_engine::project::MAX_QUESTIONS;
use toml::Table;

use super::{agent, file, strings};

fn feature_steps() -> Table {
    let workflow: Table = file("workflows/feature.toml")
        .expect("the feature workflow")
        .parse()
        .expect("the workflow TOML parses");
    workflow["steps"].as_table().expect("steps").clone()
}

#[test]
fn the_spec_loop_asks_for_answers_when_the_spec_agent_has_questions_and_for_approval_when_it_is_done()
 {
    let steps = feature_steps();
    let answer = &steps["answer-spec"];
    let approve = &steps["approve-spec"];

    assert_eq!(
        strings(&steps["spec-loop"]["body"]),
        ["spec", "answer-spec", "approve-spec"]
    );
    assert_eq!(answer["kind"].as_str(), Some("input"));
    assert_eq!(answer["artifact"].as_str(), Some("spec-questions"));
    assert_eq!(answer["answer_artifact"].as_str(), Some("spec-answers"));
    assert_eq!(strings(&answer["needs"]), ["spec"]);
    assert_eq!(answer["when"]["outcome"].as_str(), Some("questions"));
    assert_eq!(strings(&approve["needs"]), ["answer-spec"]);
    assert_eq!(approve["when"]["step"].as_str(), Some("spec"));
    assert_eq!(approve["when"]["outcome"].as_str(), Some("done"));
}

#[test]
fn the_spec_agent_delivers_the_spec_or_questions_and_continues_its_conversation() {
    let spec = agent("spec");
    let outputs = &spec["outputs"];

    assert_eq!(strings(&spec["outcomes"]), ["done", "questions"]);
    assert_eq!(strings(&outputs["spec"]["outcomes"]), ["done"]);
    assert_eq!(
        outputs["spec-questions"]["format"].as_str(),
        Some("questions")
    );
    assert_eq!(
        strings(&outputs["spec-questions"]["outcomes"]),
        ["questions"]
    );
    assert_eq!(
        spec["inputs"]["spec-answers"]["format"].as_str(),
        Some("json")
    );
    assert_eq!(
        spec["inputs"]["spec-answers"]["optional"].as_bool(),
        Some(true)
    );
    assert_eq!(spec["profile"]["conversation"].as_str(), Some("loop"));
}

#[test]
fn the_spec_instructions_bound_each_round_and_record_decisions_instead_of_open_questions() {
    let instructions = file("agents/spec/instructions.md").expect("the instructions");

    for named in [
        "`spec-answers`",
        "`questions.json`",
        "`why`",
        "`recommended`",
        "`options`",
        "outcome `questions`",
        "outcome `done`",
        "**Decisions**",
    ] {
        assert!(instructions.contains(named), "{named}");
    }
    assert!(instructions.contains(&format!("at most {MAX_QUESTIONS} questions per round")));
    assert!(!instructions.contains("Open questions"));
}
