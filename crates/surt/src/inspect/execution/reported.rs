use surt_engine::project::{AgentProfile, ExecutionDetail, ExecutionState, RuntimeInfo};

use crate::duration::exact;
use crate::symbols::SEPARATOR;

const LIST_SEPARATOR: &str = ", ";

pub(super) fn profile(profile: &AgentProfile) -> String {
    let mut parts = vec![
        profile.runtime.clone(),
        profile.model.clone(),
        format!("effort {}", profile.effort),
    ];
    parts.extend(profile.permission_mode.clone());
    parts.join(SEPARATOR)
}

pub(super) fn session(reported: &RuntimeInfo) -> Option<String> {
    joined([
        reported.session_id.clone(),
        reported
            .model
            .as_ref()
            .map(|model| format!("model {model}")),
        reported
            .version
            .as_ref()
            .map(|version| format!("version {version}")),
    ])
}

pub(super) fn process(detail: &ExecutionDetail, now: i64) -> Option<String> {
    let elapsed = match (detail.started_at, detail.finished_at) {
        (Some(started), Some(finished)) => Some(exact(finished - started)),
        (Some(started), None) if detail.state == ExecutionState::Running => {
            Some(format!("running for {}", exact(now - started)))
        }
        _ => None,
    };
    joined([
        detail.pid.map(|pid| format!("pid {pid}")),
        detail.reported.exit_code.map(|code| format!("exit {code}")),
        elapsed,
    ])
}

pub(super) fn usage(reported: &RuntimeInfo) -> Option<String> {
    let usage = reported.usage.unwrap_or_default();
    joined([
        reported.turns.map(|turns| format!("{turns} turns")),
        usage.input_tokens.map(|tokens| format!("{tokens} input")),
        usage.output_tokens.map(|tokens| format!("{tokens} output")),
        usage
            .cache_creation_input_tokens
            .map(|tokens| format!("{tokens} cache write")),
        usage
            .cache_read_input_tokens
            .map(|tokens| format!("{tokens} cache read")),
        reported.cost_usd.map(|cost| format!("${cost:.2}")),
    ])
}

pub(super) fn denied(reported: &RuntimeInfo) -> Option<String> {
    let tools: Vec<&str> = reported
        .permission_denials
        .iter()
        .map(|denial| denial["tool_name"].as_str().unwrap_or("?"))
        .collect();
    (!tools.is_empty()).then(|| tools.join(LIST_SEPARATOR))
}

fn joined<const N: usize>(parts: [Option<String>; N]) -> Option<String> {
    let parts: Vec<String> = parts.into_iter().flatten().collect();
    (!parts.is_empty()).then(|| parts.join(SEPARATOR))
}
