use serde_json::Value;
use surt_engine::project::{
    ExecutionDetail, ExecutionInput, Performer, RuntimeInfo, Timeline, TimelineEvent, Usage,
};

use crate::json::Fields;
use crate::states::execution_state_name;

pub(super) fn timeline(timeline: &Timeline) -> Fields {
    Fields::new()
        .with("subject", timeline.subject.as_str())
        .with("run", timeline.run)
        .maybe("item", timeline.item.as_deref())
        .with(
            "events",
            timeline.events.iter().map(event).collect::<Vec<_>>(),
        )
}

fn event(event: &TimelineEvent) -> Value {
    Fields::new()
        .time("at", Some(event.at))
        .with("kind", event.kind.as_str())
        .maybe("item", event.item.as_deref())
        .maybe("step", event.step.as_deref())
        .maybe("iteration", event.iteration)
        .maybe("attempt", event.attempt)
        .maybe("execution", event.execution.as_deref())
        .with("data", Value::Object(event.data.clone()))
        .into()
}

pub(super) fn execution(detail: &ExecutionDetail) -> Fields {
    let execution = Fields::new()
        .with("id", detail.id.as_str())
        .with("subject", detail.subject.as_str())
        .with("run", detail.run)
        .maybe("item", detail.item.as_deref())
        .with("step", detail.step.as_str())
        .maybe("iteration", detail.iteration)
        .with("attempt", detail.attempt)
        .with("performer", performer(&detail.performer))
        .with("state", execution_state_name(detail.state))
        .maybe("outcome", detail.outcome.as_deref())
        .maybe(
            "failure",
            detail.failure.as_ref().map(|failure| {
                Fields::new()
                    .with("class", failure.class.as_str())
                    .maybe("code", failure.code.as_deref())
                    .maybe("message", failure.message.as_deref())
            }),
        );
    Fields::new().with("execution", process(execution, detail))
}

fn process(fields: Fields, detail: &ExecutionDetail) -> Fields {
    fields
        .maybe("slot", detail.slot)
        .maybe("pid", detail.pid)
        .time("started_at", detail.started_at)
        .time("finished_at", detail.finished_at)
        .with("reported", reported(&detail.reported))
        .with(
            "inputs",
            detail.inputs.iter().map(input).collect::<Vec<_>>(),
        )
        .maybe(
            "worktree",
            detail
                .worktree
                .as_ref()
                .map(|path| path.display().to_string()),
        )
        .maybe("salvage_ref", detail.salvage_ref.as_deref())
}

fn performer(performer: &Performer) -> Fields {
    match performer {
        Performer::Agent { role, profile } => Fields::new()
            .with("kind", "agent")
            .with("role", role.as_str())
            .with("runtime", profile.runtime.as_str())
            .with("model", profile.model.as_str())
            .with("effort", profile.effort.as_str())
            .maybe("permission_mode", profile.permission_mode.as_deref()),
        Performer::Operation { id } => Fields::new()
            .with("kind", "operation")
            .with("id", id.as_str()),
    }
}

fn reported(info: &RuntimeInfo) -> Fields {
    Fields::new()
        .maybe("version", info.version.as_deref())
        .maybe("session_id", info.session_id.as_deref())
        .maybe("model", info.model.as_deref())
        .maybe("exit_code", info.exit_code)
        .maybe("turns", info.turns)
        .maybe("usage", info.usage.map(usage))
        .maybe("cost_usd", info.cost_usd)
        .list("permission_denials", info.permission_denials.clone())
        .time("resets_at", info.resets_at)
}

fn usage(usage: Usage) -> Fields {
    Fields::new()
        .maybe("input_tokens", usage.input_tokens)
        .maybe("output_tokens", usage.output_tokens)
        .maybe(
            "cache_creation_input_tokens",
            usage.cache_creation_input_tokens,
        )
        .maybe("cache_read_input_tokens", usage.cache_read_input_tokens)
}

fn input(input: &ExecutionInput) -> Value {
    Fields::new()
        .with("name", input.name.as_str())
        .with("artifact", input.artifact.as_str())
        .with("revision", input.revision)
        .with("sha256", input.sha256.as_str())
        .maybe("item", input.item.as_deref())
        .into()
}
