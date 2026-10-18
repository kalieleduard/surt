use serde_json::{Map, Value};

use crate::runtime::RuntimeReport;

const USAGE_COUNTERS: [&str; 4] = [
    "input_tokens",
    "output_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
];

pub(super) fn requested(effort: &str) -> RuntimeReport {
    RuntimeReport {
        effort: Some(effort.to_owned()),
        ..RuntimeReport::default()
    }
}

pub(super) fn initialized(report: &mut RuntimeReport, event: &Value) {
    report.session_id = text(&event["session_id"]);
    report.model = text(&event["model"]);
}

pub(super) fn finished(report: &mut RuntimeReport, event: &Value) {
    if report.session_id.is_none() {
        report.session_id = text(&event["session_id"]);
    }
    report.turns = event["num_turns"].as_u64();
    report.usage = event["usage"].as_object().map(usage);
    report.cost_usd = event["total_cost_usd"].as_f64();
    report.permission_denials = event["permission_denials"].as_array().cloned();
}

fn usage(reported: &Map<String, Value>) -> Map<String, Value> {
    USAGE_COUNTERS
        .into_iter()
        .filter_map(|counter| {
            let count = reported.get(counter)?.as_u64()?;
            Some((counter.to_owned(), Value::from(count)))
        })
        .collect()
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}
