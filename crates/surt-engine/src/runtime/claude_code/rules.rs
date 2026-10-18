use std::path::Path;

use crate::runtime::RuntimeRequest;
use crate::workflow::ClaudeProfile;

const EDIT_TOOL: &str = "Edit";
const MCP_TOOLS: &str = "mcp__*";
const VERBATIM_PREFIX: &str = r"\\?\";

pub(super) fn allowed(request: &RuntimeRequest, claude: &ClaudeProfile) -> Vec<String> {
    let edits_working_dir = claude.tools.iter().any(|tool| tool == EDIT_TOOL);
    let roots = std::iter::once(request.delivery_dir.as_path())
        .chain(edits_working_dir.then_some(request.launch.working_dir.as_path()));
    let edit_rules = roots.map(edit_rule);
    unique(claude.allowed_tools.iter().cloned().chain(edit_rules))
}

pub(super) fn disallowed(claude: &ClaudeProfile) -> Vec<String> {
    unique(
        claude
            .disallowed_tools
            .iter()
            .cloned()
            .chain(std::iter::once(MCP_TOOLS.to_owned())),
    )
}

pub(super) fn absolute_pattern(path: &str) -> String {
    let path = path.strip_prefix(VERBATIM_PREFIX).unwrap_or(path);
    let posix = match drive_letter(path) {
        Some(drive) => format!("/{}{}", drive.to_ascii_lowercase(), &path[2..]),
        None => path.to_owned(),
    };
    format!("/{}", posix.replace('\\', "/"))
}

fn edit_rule(root: &Path) -> String {
    format!("Edit({}/**)", absolute_pattern(&root.to_string_lossy()))
}

fn drive_letter(path: &str) -> Option<char> {
    let mut chars = path.chars();
    let drive = chars.next().filter(char::is_ascii_alphabetic)?;
    (chars.next() == Some(':')).then_some(drive)
}

fn unique(rules: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = Vec::new();
    for rule in rules {
        if !seen.contains(&rule) {
            seen.push(rule);
        }
    }
    seen
}
