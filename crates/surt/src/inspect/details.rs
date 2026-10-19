mod changes;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use surt_engine::project::TimelineEvent;

use super::fields::{Fields, joined};
use crate::duration::exact;
use crate::symbols::{ARROW, DASH};

const LIST_SEPARATOR: &str = ", ";
const SAME_DAY: &str = "%H:%M";
const OTHER_DAY: &str = "%Y-%m-%d %H:%M";

pub(super) fn of(event: &TimelineEvent, zone: &TimeZone) -> Vec<String> {
    let fields = Fields(&event.data);
    let kind = event.kind.as_str();
    let parts = match kind.split_once('.').map_or(kind, |(group, _)| group) {
        "run" | "driver" | "cleanup" => lifecycle(kind, fields),
        "iteration" | "step" => steps(kind, fields),
        "execution" => execution(kind, fields, event),
        "agents" => agents(kind, fields, event.at, zone),
        _ => changes::of(kind, fields),
    };
    parts.into_iter().flatten().collect()
}

fn lifecycle(kind: &str, fields: Fields<'_>) -> Vec<Option<String>> {
    match kind {
        "run.created" => vec![base(fields), fields.labelled("by", "actor")],
        "run.cancel_requested" | "run.stop_requested" => vec![fields.labelled("by", "actor")],
        "driver.started" | "driver.stopped" => vec![fields.labelled("pid", "pid")],
        "driver.recovered" => vec![
            fields.labelled("previous pid", "previous_pid"),
            found(fields),
        ],
        "cleanup.finished" | "cleanup.incomplete" => vec![kept(fields), left(fields)],
        _ => Vec::new(),
    }
}

fn steps(kind: &str, fields: Fields<'_>) -> Vec<Option<String>> {
    match kind {
        "iteration.started" => vec![iteration(fields)],
        "step.done" => vec![fields.text("outcome").map(str::to_owned)],
        "step.blocked" => vec![fields.text("reason").map(str::to_owned)],
        "step.unblocked" => vec![fields.labelled("budget_base", "budget_base")],
        _ => Vec::new(),
    }
}

fn execution(kind: &str, fields: Fields<'_>, event: &TimelineEvent) -> Vec<Option<String>> {
    match kind {
        "execution.started" => vec![
            fields.number("slot").map(|slot| format!("agent-{slot}")),
            fields.text("runtime").map(str::to_owned),
            fields.labelled("operation", "operation"),
            event.execution.clone(),
        ],
        "execution.succeeded" => vec![fields.text("outcome").map(str::to_owned)],
        "execution.failed" => vec![joined(
            [fields.text("code"), fields.text("message")].map(|part| part.map(str::to_owned)),
            &format!(" {DASH} "),
        )],
        "execution.orphan_stopped" => vec![fields.labelled("pid", "pid")],
        _ => Vec::new(),
    }
}

fn agents(kind: &str, fields: Fields<'_>, at: i64, zone: &TimeZone) -> Vec<Option<String>> {
    match kind {
        "agents.paused" => vec![
            fields
                .number("until")
                .and_then(|until| clock(until, at, zone))
                .map(|until| format!("until {until}")),
            fields.text("code").map(str::to_owned),
            fields.labelled("backoff step", "backoff_step"),
            fields
                .number("waiting")
                .map(|waiting| format!("{waiting} waiting")),
        ],
        "agents.resumed" => vec![
            fields
                .number("paused_seconds")
                .map(|seconds| format!("after {}", exact(seconds))),
        ],
        _ => Vec::new(),
    }
}

fn clock(seconds: i64, at: i64, zone: &TimeZone) -> Option<String> {
    let time = Timestamp::from_second(seconds).ok()?.to_zoned(zone.clone());
    let event = Timestamp::from_second(at).ok()?.to_zoned(zone.clone());
    let format = if time.date() == event.date() {
        SAME_DAY
    } else {
        OTHER_DAY
    };
    Some(time.strftime(format).to_string())
}
fn base(fields: Fields<'_>) -> Option<String> {
    let base = fields.object("base")?;
    let at = base.sha("sha").map(|sha| format!("@ {sha}"));
    joined(
        [
            Some("base".to_owned()),
            base.text("ref").map(str::to_owned),
            at,
        ],
        " ",
    )
}

fn found(fields: Fields<'_>) -> Option<String> {
    let findings: Vec<String> = fields
        .list("found")
        .iter()
        .filter_map(|finding| finding.as_object().map(Fields))
        .filter_map(|finding| {
            joined(
                [
                    finding.text("kind").map(str::to_owned),
                    finding.labelled("item", "item"),
                    finding.text("step").map(str::to_owned),
                ],
                " ",
            )
        })
        .collect();
    joined(findings.into_iter().map(Some), LIST_SEPARATOR).map(|found| format!("found {found}"))
}

fn iteration(fields: Fields<'_>) -> Option<String> {
    let until = match (fields.text("cause"), fields.text("step")) {
        (Some("until_not_met"), Some(step)) => Some(match fields.text("outcome") {
            Some(outcome) => format!("until {step} not met: {outcome}"),
            None => format!("until {step} not met"),
        }),
        _ => None,
    };
    joined([fields.labelled("iter", "iteration"), until], " ")
}

fn kept(fields: Fields<'_>) -> Option<String> {
    let kept = fields.list("kept").iter().filter_map(|entry| {
        let entry = Fields(entry.as_object()?);
        entry
            .text("path")
            .or(entry.text("branch"))
            .map(str::to_owned)
    });
    joined(kept.map(Some), LIST_SEPARATOR).map(|kept| format!("kept {kept}"))
}

fn left(fields: Fields<'_>) -> Option<String> {
    let left = fields.list("left").iter().filter_map(|entry| {
        let entry = Fields(entry.as_object()?);
        joined(
            [entry.text("item"), entry.text("reason")].map(|part| part.map(str::to_owned)),
            ": ",
        )
    });
    joined(left.map(Some), LIST_SEPARATOR).map(|left| format!("left {left}"))
}

pub(super) fn moved(fields: Fields<'_>) -> Option<String> {
    Some(format!(
        "{} {ARROW} {}",
        fields.sha("old_sha")?,
        fields.sha("new_sha")?
    ))
}
