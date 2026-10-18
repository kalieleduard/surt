use std::collections::BTreeSet;

use serde_json::Value;
use uuid::Uuid;

use super::invocation::Expectation;
use crate::runtime::{FailureClass, Initialization, RuntimeError};

const MINIMUM_VERSION: [u32; 3] = [2, 1, 301];
const NO_API_KEY: &str = "none";

pub(super) fn check(
    expectation: &Expectation,
    event: &Value,
) -> Result<Initialization, RuntimeError> {
    let session_id = event["session_id"]
        .as_str()
        .and_then(|session| Uuid::parse_str(session).ok());
    let model = event["model"].as_str().unwrap_or_default();
    if session_id != Some(expectation.session_id) {
        return Err(RuntimeError::new(
            FailureClass::Retryable,
            "session_mismatch",
            format!(
                "Claude Code initialized session {} instead of {}",
                event["session_id"], expectation.session_id
            ),
        ));
    }
    if model != expectation.model {
        return Err(RuntimeError::new(
            FailureClass::NotRetryable,
            "model_mismatch",
            format!(
                "Claude Code initialized model {model} instead of {}",
                expectation.model
            ),
        ));
    }
    check_version(event)?;
    check_isolation(expectation, event)?;
    Ok(Initialization {
        session_id: expectation.session_id,
        model: model.to_owned(),
    })
}

fn check_version(event: &Value) -> Result<(), RuntimeError> {
    let reported = event["claude_code_version"].as_str().unwrap_or_default();
    match version(reported) {
        Some(version) if version >= MINIMUM_VERSION => Ok(()),
        _ => Err(RuntimeError::new(
            FailureClass::NotRetryable,
            "unsupported_version",
            format!("Claude Code {reported:?} is older than the supported 2.1.301; upgrade it"),
        )),
    }
}

fn version(text: &str) -> Option<[u32; 3]> {
    let core = text.split(['-', '+', ' ']).next()?;
    let mut parts = core.split('.').map(|part| part.parse::<u32>().ok());
    let version = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(version)
}

fn check_isolation(expectation: &Expectation, event: &Value) -> Result<(), RuntimeError> {
    let tools: Option<BTreeSet<String>> = event["tools"].as_array().map(|tools| {
        tools
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    });
    let api_key_source = event["apiKeySource"].as_str().unwrap_or(NO_API_KEY);
    let mismatch = if event["permissionMode"] != expectation.permission_mode {
        Some(format!(
            "permission mode {} instead of {}",
            event["permissionMode"], expectation.permission_mode
        ))
    } else if tools.as_ref() != Some(&expectation.tools) {
        Some(format!(
            "tools {} instead of {:?}",
            event["tools"], expectation.tools
        ))
    } else if event["mcp_servers"]
        .as_array()
        .is_none_or(|servers| !servers.is_empty())
    {
        Some(format!("MCP servers {}", event["mcp_servers"]))
    } else if expectation.subscription && api_key_source != NO_API_KEY {
        Some(format!(
            "API key source {api_key_source} instead of the subscription login"
        ))
    } else {
        None
    };
    mismatch.map_or(Ok(()), |mismatch| {
        Err(RuntimeError::new(
            FailureClass::NotRetryable,
            "initialization_mismatch",
            format!("Claude Code initialized with {mismatch}"),
        ))
    })
}
