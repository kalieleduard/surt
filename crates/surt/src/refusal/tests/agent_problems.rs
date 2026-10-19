use surt_engine::project::Problem;

use super::problems::{owned, problem_line};

#[test]
fn every_agent_problem_is_described_in_one_line_of_words() {
    let cases = [
        (
            Problem::MissingAsset {
                path: owned("schemas/x.json"),
            },
            "\"schemas/x.json\" is not a file in the agent's directory",
        ),
        (
            Problem::NonLocalReference {
                reference: owned("https://example.com/x.json"),
            },
            "$ref \"https://example.com/x.json\" is not a file in the agent's schemas directory",
        ),
        (
            Problem::InvalidSchema {
                detail: owned("expected value at line 1 column 3"),
            },
            "not a valid JSON Schema: expected value at line 1 column 3",
        ),
        (
            Problem::MisplacedJsonSchema { name: owned("x") },
            "\"x\" has a json_schema but is neither json nor item_list",
        ),
        (
            Problem::MisplacedItemFiles { name: owned("x") },
            "\"x\" declares item_files but is not an item_list",
        ),
        (
            Problem::UnsupportedItemFileFormat { name: owned("x") },
            "item file \"x\" has an unsupported format",
        ),
        (
            Problem::MissingClaudeProfile,
            "runtime claude-code needs a [profile.claude] table",
        ),
        (
            Problem::MisplacedApiKeyEnv,
            "api_key_env needs auth = \"api_key\"",
        ),
        (
            Problem::InvalidBudget,
            "max_budget_usd must be a positive number",
        ),
        (Problem::MissingFile, "the file does not exist"),
        (
            Problem::StrayAgentFile,
            "surt reads only agent.toml, instructions.md and schemas/ here",
        ),
        (Problem::TestOnlyRuntime, "runtime \"fake\" is test-only"),
        (
            Problem::MissingCheckedOutput {
                output: owned("commit"),
            },
            "a delivery check needs the JSON output \"commit\"",
        ),
        (
            Problem::MissingCheckedOutcome {
                outcome: owned("approved"),
            },
            "a delivery check needs the outcome \"approved\"",
        ),
    ];

    for (problem, expected) in cases {
        assert_eq!(problem_line(problem), expected);
    }
}
