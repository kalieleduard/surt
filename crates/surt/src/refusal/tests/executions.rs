use surt_engine::project::{ExecutionCandidate, Refusal};

use super::text;

const SUBJECT: &str = "20261017-add-login";
const FIRST: &str = "0192f3a8-7b1c-7000-8000-000000000001";
const SECOND: &str = "0192f3a8-9d20-7000-8000-000000000002";

fn candidate(id: &str, item: Option<&str>, attempt: u32, state: &str) -> ExecutionCandidate {
    ExecutionCandidate {
        id: id.to_owned(),
        subject: SUBJECT.to_owned(),
        run: 1,
        item: item.map(str::to_owned),
        step: "build".to_owned(),
        iteration: Some(2),
        attempt,
        state: state.to_owned(),
    }
}

#[test]
fn an_ambiguous_prefix_lists_each_execution_with_where_it_ran_and_its_state() {
    let refusal = Refusal::AmbiguousExecution {
        reference: "0192f3a8".to_owned(),
        candidates: vec![
            candidate(FIRST, Some("B"), 1, "failed"),
            candidate(SECOND, Some("B"), 2, "succeeded"),
        ],
    };

    assert_eq!(
        text(&refusal),
        format!(
            "✖ \"0192f3a8\" matches 2 executions:\n  \
             \x20 {FIRST}  {SUBJECT} · run 1 · task B · build iter 2 · attempt 1  failed\n  \
             \x20 {SECOND}  {SUBJECT} · run 1 · task B · build iter 2 · attempt 2  succeeded\n  \
             use the full id, as surt prints it\n"
        )
    );
}

#[test]
fn a_too_short_prefix_says_how_many_characters_it_needs_and_lists_at_most_ten() {
    let candidates: Vec<ExecutionCandidate> = (1..=12)
        .map(|attempt| candidate(&format!("0192f3a8-{attempt:04}"), None, attempt, "failed"))
        .collect();
    let refusal = Refusal::ExecutionPrefixTooShort {
        reference: "0192".to_owned(),
        minimum: 8,
        candidates,
    };

    let text = text(&refusal);

    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0],
        "✖ \"0192\" is too short; an execution id prefix needs at least 8 characters"
    );
    assert_eq!(
        lines[1],
        format!("    0192f3a8-0001  {SUBJECT} · run 1 · build iter 2 · attempt 1   failed")
    );
    assert_eq!(lines.len(), 13, "{text}");
    assert_eq!(lines[11], "    … and 2 more");
    assert_eq!(lines[12], "  use the full id, as surt prints it");
}

#[test]
fn an_unknown_reference_names_both_things_it_could_have_been() {
    let refusal = Refusal::UnknownExecution {
        reference: "ffffffff".to_owned(),
    };

    assert_eq!(
        text(&refusal),
        "✖ no feature or execution matches \"ffffffff\"\n  inspect: surt status\n"
    );
}
