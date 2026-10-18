use std::ffi::{OsStr, OsString};

use crate::runtime::{FailureClass, RuntimeError};
use crate::workflow::{Auth, ClaudeProfile};

const API_KEY: &str = "ANTHROPIC_API_KEY";
const ENABLED: &str = "1";

const INHERITED: &[&str] = &[
    "HOME",
    "PATH",
    "USER",
    "LOGNAME",
    "SHELL",
    "TMPDIR",
    "TZ",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "LC_MESSAGES",
    "CLAUDE_CONFIG_DIR",
    "SYSTEMROOT",
    "SYSTEMDRIVE",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "USERNAME",
    "USERDOMAIN",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "TEMP",
    "TMP",
    "CLAUDE_CODE_GIT_BASH_PATH",
];

const ISOLATION: &[&str] = &[
    "DISABLE_AUTOUPDATER",
    "DISABLE_UPDATES",
    "CLAUDE_CODE_DISABLE_AUTO_MEMORY",
    "CLAUDE_CODE_DISABLE_CLAUDE_MDS",
    "CLAUDE_CODE_STARTUP_FAILURE_RESULTS",
];

pub(super) fn child_environment(
    parent: &[(OsString, OsString)],
    claude: &ClaudeProfile,
) -> Result<Vec<(OsString, OsString)>, RuntimeError> {
    let inherited = parent
        .iter()
        .filter(|(name, _)| INHERITED.iter().any(|allowed| same_name(name, allowed)))
        .cloned();
    let isolation = ISOLATION
        .iter()
        .map(|name| (OsString::from(name), OsString::from(ENABLED)));
    let api_key = api_key(parent, claude)?.map(|key| (OsString::from(API_KEY), key));
    Ok(inherited.chain(isolation).chain(api_key).collect())
}

fn api_key(
    parent: &[(OsString, OsString)],
    claude: &ClaudeProfile,
) -> Result<Option<OsString>, RuntimeError> {
    if claude.auth != Auth::ApiKey {
        return Ok(None);
    }
    let variable = claude.api_key_env.as_deref().unwrap_or(API_KEY);
    parent
        .iter()
        .find(|(name, _)| name == variable)
        .map(|(_, key)| Some(key.clone()))
        .ok_or_else(|| {
            RuntimeError::new(
                FailureClass::NotRetryable,
                "missing_api_key",
                format!("the api_key profile reads its key from {variable}, which is not set"),
            )
        })
}

fn same_name(name: &OsStr, allowed: &str) -> bool {
    name.to_str()
        .is_some_and(|name| name.eq_ignore_ascii_case(allowed))
}
