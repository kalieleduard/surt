use surt_engine::project::MAX_QUESTIONS;

use super::spec::feature_steps;
use super::{agent, file, strings};

#[test]
fn the_discovery_loop_comes_first_and_asks_for_answers_until_the_agent_is_done() {
    let steps = feature_steps();
    let answer = &steps["answer-discovery"];
    let discovery_loop = &steps["discovery-loop"];

    assert_eq!(steps.keys().next().map(String::as_str), Some("discovery"));
    assert_eq!(
        strings(&discovery_loop["body"]),
        ["discovery", "answer-discovery"]
    );
    assert_eq!(discovery_loop["until"]["step"].as_str(), Some("discovery"));
    assert_eq!(discovery_loop["until"]["outcome"].as_str(), Some("done"));
    assert_eq!(answer["kind"].as_str(), Some("input"));
    assert_eq!(answer["artifact"].as_str(), Some("discovery-questions"));
    assert_eq!(
        answer["answer_artifact"].as_str(),
        Some("discovery-answers")
    );
    assert_eq!(strings(&answer["needs"]), ["discovery"]);
    assert_eq!(answer["when"]["outcome"].as_str(), Some("questions"));
}

#[test]
fn the_discovery_agent_delivers_the_brief_or_questions_and_continues_its_conversation() {
    let discovery = agent("discovery");
    let outputs = &discovery["outputs"];

    assert_eq!(strings(&discovery["outcomes"]), ["done", "questions"]);
    assert_eq!(outputs["brief"]["format"].as_str(), Some("markdown"));
    assert_eq!(strings(&outputs["brief"]["outcomes"]), ["done"]);
    assert_eq!(
        outputs["discovery-questions"]["format"].as_str(),
        Some("questions")
    );
    assert_eq!(
        strings(&outputs["discovery-questions"]["outcomes"]),
        ["questions"]
    );
    assert_eq!(
        discovery["inputs"]["discovery-answers"]["optional"].as_bool(),
        Some(true)
    );
    assert_eq!(discovery["profile"]["conversation"].as_str(), Some("loop"));
}

#[test]
fn the_discovery_instructions_bound_each_round_and_keep_the_questions_plain() {
    let instructions = file("agents/discovery/instructions.md").expect("the instructions");

    for named in [
        "`discovery-answers`",
        "`questions.json`",
        "`why`",
        "`recommended`",
        "`options`",
        "`description`",
        "WebSearch",
        "outcome `questions`",
        "outcome `done`",
        "plain, everyday words",
    ] {
        assert!(instructions.contains(named), "{named}");
    }
    assert!(instructions.contains(&format!("at most {MAX_QUESTIONS} questions per round")));
}
