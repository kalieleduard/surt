use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct RuntimeInfo {
    #[serde(default, rename = "runtime_version")]
    pub version: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i64>,
    #[serde(default)]
    pub turns: Option<u64>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub permission_denials: Vec<Value>,
    #[serde(default)]
    pub resets_at: Option<i64>,
    #[serde(default)]
    pub conversation: Option<ConversationInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum ConversationInfo {
    Continued {
        #[serde(rename = "continued_from")]
        from: String,
    },
    Restarted {
        #[serde(rename = "restarted")]
        reason: String,
        previous: String,
    },
    Fresh {
        #[serde(rename = "fresh")]
        reason: String,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

impl RuntimeInfo {
    pub(super) fn parse(text: Option<&str>) -> RuntimeInfo {
        text.and_then(|text| serde_json::from_str(text).ok())
            .unwrap_or_default()
    }
}
