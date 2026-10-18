use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::PathBuf;

use serde_json::{Map, Value, json};
use uuid::Uuid;

use super::{environment, rules};
use crate::runtime::{FailureClass, RuntimeError, RuntimeRequest};
use crate::workflow::{Auth, ClaudeProfile, Effort, PermissionMode};

const STRUCTURED_OUTPUT_TOOL: &str = "StructuredOutput";
const INSTRUCTIONS_FILE: &str = "instructions.md";
const SETTINGS_FILE: &str = "settings.json";
const MCP_FILE: &str = "mcp.json";

#[derive(Debug)]
pub(super) struct Invocation {
    pub(super) arguments: Vec<OsString>,
    pub(super) environment: Vec<(OsString, OsString)>,
    pub(super) files: Vec<(PathBuf, String)>,
    pub(super) working_dir: PathBuf,
    pub(super) stdin: String,
    pub(super) expectation: Expectation,
}

#[derive(Clone, Debug)]
pub(super) struct Expectation {
    pub(super) session_id: Uuid,
    pub(super) model: String,
    pub(super) permission_mode: &'static str,
    pub(super) tools: BTreeSet<String>,
    pub(super) subscription: bool,
}

impl Invocation {
    pub(super) fn assemble(
        request: &RuntimeRequest,
        parent_environment: &[(OsString, OsString)],
    ) -> Result<Invocation, RuntimeError> {
        let claude = claude_profile(request)?;
        let launch = &request.launch;
        let files = vec![
            (
                launch.launch_dir.join(INSTRUCTIONS_FILE),
                launch.instructions.clone(),
            ),
            (launch.launch_dir.join(SETTINGS_FILE), settings()),
            (
                launch.launch_dir.join(MCP_FILE),
                json!({ "mcpServers": {} }).to_string(),
            ),
        ];
        Ok(Invocation {
            arguments: arguments(request, claude),
            environment: environment::child_environment(parent_environment, claude)?,
            files,
            working_dir: launch.working_dir.clone(),
            stdin: request.context.clone(),
            expectation: Expectation::new(request, claude),
        })
    }
}

impl Expectation {
    fn new(request: &RuntimeRequest, claude: &ClaudeProfile) -> Expectation {
        let tools = claude
            .tools
            .iter()
            .map(String::as_str)
            .chain([STRUCTURED_OUTPUT_TOOL])
            .map(str::to_owned)
            .collect();
        Expectation {
            session_id: request.session_id,
            model: request.model.clone(),
            permission_mode: permission_mode(claude.permission_mode),
            tools,
            subscription: claude.auth == Auth::Subscription,
        }
    }
}

fn arguments(request: &RuntimeRequest, claude: &ClaudeProfile) -> Vec<OsString> {
    let launch = &request.launch;
    let text = |values: &[&str]| values.iter().map(OsString::from).collect::<Vec<_>>();
    let mut arguments = text(&[
        "-p",
        "--session-id",
        &request.session_id.to_string(),
        "--no-session-persistence",
        "--output-format",
        "stream-json",
        "--verbose",
        "--model",
        &request.model,
        "--effort",
        effort(launch.profile.effort),
        "--permission-mode",
        permission_mode(claude.permission_mode),
        "--permission-prompts",
        "none",
        "--tools",
        &claude.tools.join(","),
        "--allowedTools",
    ]);
    arguments.extend(
        rules::allowed(request, claude)
            .into_iter()
            .map(OsString::from),
    );
    arguments.push("--disallowedTools".into());
    arguments.extend(rules::disallowed(claude).into_iter().map(OsString::from));
    arguments.extend([
        "--json-schema".into(),
        envelope_schema(&launch.outcomes, &launch.outputs).into(),
        "--append-system-prompt-file".into(),
        launch.launch_dir.join(INSTRUCTIONS_FILE).into(),
        "--settings".into(),
        launch.launch_dir.join(SETTINGS_FILE).into(),
        "--strict-mcp-config".into(),
        "--mcp-config".into(),
        launch.launch_dir.join(MCP_FILE).into(),
        "--disable-slash-commands".into(),
    ]);
    arguments.extend(limits(request));
    arguments.extend(text(auth_arguments(claude.auth)));
    arguments
}

fn limits(request: &RuntimeRequest) -> Vec<OsString> {
    let Some(limits) = request.launch.profile.limits else {
        return Vec::new();
    };
    let turns = limits
        .max_turns
        .map(|turns| ["--max-turns".to_owned(), turns.to_string()]);
    let budget = limits
        .max_budget_usd
        .map(|usd| ["--max-budget-usd".to_owned(), usd.to_string()]);
    turns
        .into_iter()
        .chain(budget)
        .flatten()
        .map(OsString::from)
        .collect()
}

fn auth_arguments(auth: Auth) -> &'static [&'static str] {
    match auth {
        Auth::Subscription => &["--safe-mode", "--setting-sources", "user"],
        Auth::ApiKey => &["--bare"],
    }
}

fn envelope_schema(outcomes: &[String], outputs: &[String]) -> String {
    let properties: Map<String, Value> = outputs
        .iter()
        .map(|name| (name.clone(), json!({ "type": "string" })))
        .collect();
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["outcome", "outputs"],
        "properties": {
            "outcome": { "enum": outcomes },
            "outputs": {
                "type": "object",
                "additionalProperties": false,
                "required": outputs,
                "properties": properties
            }
        }
    })
    .to_string()
}

fn settings() -> String {
    json!({ "disableAllHooks": true, "switchModelsOnFlag": false }).to_string()
}

fn claude_profile(request: &RuntimeRequest) -> Result<&ClaudeProfile, RuntimeError> {
    request.launch.profile.claude.as_ref().ok_or_else(|| {
        RuntimeError::new(
            FailureClass::NotRetryable,
            "unsupported_profile",
            "the agent profile has no [profile.claude] table for the claude-code runtime",
        )
    })
}

fn permission_mode(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::DontAsk => "dontAsk",
        PermissionMode::BypassPermissions => "bypassPermissions",
    }
}

fn effort(effort: Effort) -> &'static str {
    match effort {
        Effort::Low => "low",
        Effort::Medium => "medium",
        Effort::High => "high",
        Effort::Xhigh => "xhigh",
        Effort::Max => "max",
    }
}
