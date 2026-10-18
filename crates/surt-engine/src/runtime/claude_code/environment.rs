use std::ffi::{OsStr, OsString};

use crate::platform;
use crate::runtime::{FailureClass, RuntimeError};
use crate::workflow::{Auth, ClaudeProfile};

const API_KEY: &str = "ANTHROPIC_API_KEY";
const ENABLED: &str = "1";
const GIT_CONFIG_COUNT: &str = "GIT_CONFIG_COUNT";
const GIT_CONFIG_KEY: &str = "GIT_CONFIG_KEY_";
const GIT_CONFIG_VALUE: &str = "GIT_CONFIG_VALUE_";

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
    Ok(inherited
        .chain(isolation)
        .chain(api_key)
        .chain(git_config(parent, platform::GIT_SETTINGS))
        .collect())
}

pub(super) fn git_config(
    parent: &[(OsString, OsString)],
    appended: &[(&str, &str)],
) -> Vec<(OsString, OsString)> {
    let variable = |name: &str| {
        parent
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    };
    let inherited = variable(GIT_CONFIG_COUNT)
        .and_then(|count| count.to_str()?.parse::<usize>().ok())
        .unwrap_or_default();
    if inherited == 0 && appended.is_empty() {
        return Vec::new();
    }
    let mut settings = Vec::new();
    for index in 0..inherited {
        for prefix in [GIT_CONFIG_KEY, GIT_CONFIG_VALUE] {
            let name = format!("{prefix}{index}");
            if let Some(value) = variable(&name) {
                settings.push((OsString::from(name), value.clone()));
            }
        }
    }
    for (offset, (key, value)) in appended.iter().enumerate() {
        let index = inherited + offset;
        settings.push((format!("{GIT_CONFIG_KEY}{index}").into(), (*key).into()));
        settings.push((format!("{GIT_CONFIG_VALUE}{index}").into(), (*value).into()));
    }
    let count = inherited + appended.len();
    settings.push((GIT_CONFIG_COUNT.into(), count.to_string().into()));
    settings
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
