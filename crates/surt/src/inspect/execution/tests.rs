use std::path::PathBuf;

use serde_json::json;
use surt_engine::project::{
    AgentProfile, ExecutionDetail, ExecutionFailure, ExecutionInput, ExecutionState, Performer,
    RuntimeInfo, Usage,
};

use super::render;
use crate::driving::local_time;
use crate::symbols::Symbols;

const ID: &str = "0192f3a8-7b1c-7000-8000-000000000001";
const SUBJECT: &str = "20261017-add-login";
const STARTED: i64 = 1_760_702_400;
const SHA: &str = "3f9a0c1d2e3f405162738495a6b7c8d9e0f1a2b3c4d5e6f708192a3b4c5d6e7f";
const NOW: i64 = STARTED + 3_600;

fn input(name: &str, artifact: &str, revision: u32, item: Option<&str>) -> ExecutionInput {
    ExecutionInput {
        name: name.to_owned(),
        artifact: artifact.to_owned(),
        revision,
        sha256: SHA.to_owned(),
        item: item.map(str::to_owned),
    }
}

fn bare() -> ExecutionDetail {
    ExecutionDetail {
        id: ID.to_owned(),
        subject: SUBJECT.to_owned(),
        run: 1,
        item: None,
        step: "write".to_owned(),
        iteration: None,
        attempt: 1,
        performer: Performer::Agent {
            role: "writer".to_owned(),
            profile: AgentProfile {
                runtime: "fake".to_owned(),
                model: "fake-model".to_owned(),
                effort: "high".to_owned(),
                permission_mode: None,
            },
        },
        state: ExecutionState::Succeeded,
        outcome: Some("done".to_owned()),
        failure: None,
        slot: None,
        pid: None,
        started_at: None,
        finished_at: None,
        reported: RuntimeInfo::default(),
        inputs: Vec::new(),
        worktree: None,
        salvage_ref: None,
    }
}

fn failed_builder() -> ExecutionDetail {
    ExecutionDetail {
        run: 2,
        item: Some("B".to_owned()),
        step: "build".to_owned(),
        iteration: Some(2),
        performer: Performer::Agent {
            role: "builder".to_owned(),
            profile: AgentProfile {
                runtime: "claude-code".to_owned(),
                model: "claude-sonnet-5-5".to_owned(),
                effort: "high".to_owned(),
                permission_mode: Some("dontAsk".to_owned()),
            },
        },
        state: ExecutionState::Failed,
        outcome: None,
        failure: Some(ExecutionFailure {
            class: "retryable".to_owned(),
            code: Some("invalid_delivery".to_owned()),
            message: Some("final JSON missing \"outputs\"".to_owned()),
        }),
        slot: Some(0),
        pid: Some(50_122),
        started_at: Some(STARTED),
        finished_at: Some(STARTED + 449),
        reported: RuntimeInfo {
            version: Some("2.1.0".to_owned()),
            session_id: Some("7c1e9d2a-0000-4000-8000-000000000002".to_owned()),
            model: Some("claude-sonnet-5-5-20261001".to_owned()),
            exit_code: Some(0),
            turns: Some(12),
            usage: Some(Usage {
                input_tokens: Some(1_200),
                output_tokens: Some(300),
                cache_creation_input_tokens: None,
                cache_read_input_tokens: Some(9_000),
            }),
            cost_usd: Some(0.4213),
            permission_denials: vec![
                json!({ "tool_name": "Write" }),
                json!({ "tool_name": "Bash" }),
            ],
            resets_at: None,
        },
        inputs: vec![
            input("item", "plan", 1, Some("B")),
            input("request", "request", 1, None),
            input("spec", "spec", 2, None),
        ],
        worktree: Some(PathBuf::from(
            "/home/dev/.surt/worktrees/p/20261017-add-login.2/B",
        )),
        salvage_ref: Some(format!("refs/surt/salvage/{ID}")),
        ..bare()
    }
}

#[test]
fn a_failed_builder_shows_everything_one_screen_needs() {
    let text = render(&failed_builder(), NOW);

    assert_eq!(
        text,
        format!(
            "execution {ID}\n\
             \x20 feature   {SUBJECT} · run 2 · task B\n\
             \x20 step      build iter 2 · attempt 1 · agent-0\n\
             \x20 role      builder\n\
             \x20 state     failed (retryable) · invalid_delivery — final JSON missing \"outputs\"\n\
             \x20 profile   claude-code · claude-sonnet-5-5 · effort high · dontAsk\n\
             \x20 session   7c1e9d2a-0000-4000-8000-000000000002 · model claude-sonnet-5-5-20261001 · version 2.1.0\n\
             \x20 process   pid 50122 · exit 0 · 7m29s\n\
             \x20 usage     12 turns · 1200 input · 300 output · 9000 cache read · $0.42\n\
             \x20 denied    Write, Bash\n\
             \x20 inputs    item B (plan rev 1) sha256:3f9a0c1d2e3f…\n\
             \x20           request rev 1 sha256:3f9a0c1d2e3f…\n\
             \x20           spec rev 2 sha256:3f9a0c1d2e3f…\n\
             \x20 worktree  /home/dev/.surt/worktrees/p/20261017-add-login.2/B\n\
             \x20 salvage   refs/surt/salvage/{ID}\n\
             \x20 logs      surt logs {ID}\n"
        )
    );
}

#[test]
fn values_the_runtime_did_not_report_are_left_out() {
    let text = render(&bare(), NOW);

    assert_eq!(
        text,
        format!(
            "execution {ID}\n\
             \x20 feature  {SUBJECT} · run 1\n\
             \x20 step     write · attempt 1\n\
             \x20 role     writer\n\
             \x20 state    succeeded (done)\n\
             \x20 profile  fake · fake-model · effort high\n\
             \x20 logs     surt logs {ID}\n"
        )
    );
}

#[test]
fn a_running_execution_shows_how_long_it_has_run_and_a_zero_is_still_shown() {
    let running = ExecutionDetail {
        state: ExecutionState::Running,
        outcome: None,
        started_at: Some(STARTED),
        reported: RuntimeInfo {
            turns: Some(0),
            cost_usd: Some(0.0),
            ..RuntimeInfo::default()
        },
        ..bare()
    };

    let text = render(&running, STARTED + 3_725);

    assert!(text.contains("  state    running\n"), "{text}");
    assert!(text.contains("  process  running for 1h02m05s\n"), "{text}");
    assert!(text.contains("  usage    0 turns · $0.00\n"), "{text}");
}

#[test]
fn an_operation_is_named_instead_of_a_role_and_profile() {
    let check = ExecutionDetail {
        performer: Performer::Operation {
            id: "checks.run".to_owned(),
        },
        state: ExecutionState::Succeeded,
        outcome: Some("passed".to_owned()),
        ..bare()
    };

    let text = render(&check, NOW);

    assert!(text.contains("  operation  checks.run\n"), "{text}");
    assert!(!text.contains("profile"), "{text}");
}

#[test]
fn an_execution_prints_only_ascii_in_ascii_mode() {
    for detail in [failed_builder(), bare()] {
        let text = render(&detail, NOW);

        assert!(Symbols::Ascii.render(&text).is_ascii(), "{text}");
    }
}

#[test]
fn a_rate_limited_execution_shows_its_class_its_code_and_the_reset_time() {
    let reset = STARTED + 7_200;
    let at = local_time(reset).expect("a local time");
    let limited = ExecutionDetail {
        failure: Some(ExecutionFailure {
            class: "transient".to_owned(),
            code: Some("rate_limit".to_owned()),
            message: Some("rate limited".to_owned()),
        }),
        reported: RuntimeInfo {
            resets_at: Some(reset),
            ..RuntimeInfo::default()
        },
        ..failed_builder()
    };

    let text = render(&limited, NOW);

    assert!(
        text.contains(&format!(
            "  state     failed (transient) · rate_limit — rate limited · resets {at}\n"
        )),
        "{text}"
    );
}
