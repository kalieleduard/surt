use jiff::Zoned;
use jiff::civil::date;
use jiff::tz::TimeZone;
use serde_json::{Map, Value, json};
use surt_engine::project::{Timeline, TimelineEvent};

use super::render;
use crate::symbols::Symbols;

const SUBJECT: &str = "20261017-add-login";
const NOON: i64 = 1_760_702_400;
const EXECUTION: &str = "0192f3a8-7b1c-7000-8000-000000000001";

fn now() -> Zoned {
    date(2025, 10, 17)
        .at(18, 0, 0, 0)
        .to_zoned(TimeZone::UTC)
        .expect("a UTC time")
}

fn data(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

fn event(at: i64, kind: &str, value: Value) -> TimelineEvent {
    TimelineEvent {
        at,
        kind: kind.to_owned(),
        item: None,
        step: None,
        iteration: None,
        attempt: None,
        execution: None,
        data: data(value),
    }
}

fn in_step(mut event: TimelineEvent, step: &str, iteration: Option<u32>) -> TimelineEvent {
    event.step = Some(step.to_owned());
    event.iteration = iteration;
    event
}

fn of_execution(event: TimelineEvent, step: &str, iteration: u32, attempt: u32) -> TimelineEvent {
    let mut event = in_step(event, step, Some(iteration));
    event.attempt = Some(attempt);
    event.execution = Some(EXECUTION.to_owned());
    event
}

fn of_task(mut event: TimelineEvent, item: &str) -> TimelineEvent {
    event.item = Some(item.to_owned());
    event
}

fn timeline(item: Option<&str>, events: Vec<TimelineEvent>) -> Timeline {
    Timeline {
        subject: SUBJECT.to_owned(),
        run: 1,
        item: item.map(str::to_owned),
        events,
    }
}

fn shown(timeline: &Timeline) -> String {
    render(timeline, &now())
}

#[test]
fn the_run_timeline_prints_each_event_with_its_time_and_details() {
    let events = vec![
        event(
            NOON,
            "run.created",
            json!({ "base": { "ref": "main", "sha": "a11c9d0e5f60" }, "actor": "you@example.com" }),
        ),
        of_execution(
            event(
                NOON + 60,
                "revision.published",
                json!({ "artifact": "spec", "revision": 1, "origin": "execution" }),
            ),
            "write",
            1,
            1,
        ),
        in_step(
            event(
                NOON + 120,
                "approval.recorded",
                json!({ "decision": "reject", "artifact": "spec", "revision": 1, "actor": "you@example.com", "reason": "needs OAuth" }),
            ),
            "approve_spec",
            Some(1),
        ),
        event(NOON + 180, "driver.started", json!({ "pid": 48213 })),
        of_task(
            event(NOON + 240, "run.created", json!({ "item": "B" })),
            "B",
        ),
    ];

    assert_eq!(
        shown(&timeline(None, events)),
        "12:00:00  run.created         base main @ a11c9d0 · by you@example.com\n\
         12:01:00  revision.published  write iter 1 · attempt 1 · spec rev 1 (agent)\n\
         12:02:00  approval.recorded   approve_spec iter 1 · spec rev 1 rejected — \"needs OAuth\" · by you@example.com\n\
         12:03:00  driver.started      pid 48213\n\
         12:04:00  run.created         task B\n"
    );
}

#[test]
fn the_task_timeline_explains_iterations_attempts_and_blocks() {
    let events = vec![
        in_step(
            event(
                NOON,
                "iteration.started",
                json!({ "iteration": 2, "cause": "until_not_met", "step": "review", "outcome": "changes_requested" }),
            ),
            "work",
            None,
        ),
        in_step(event(NOON + 1, "step.running", json!({})), "build", Some(2)),
        of_execution(
            event(
                NOON + 2,
                "execution.started",
                json!({ "attempt": 1, "slot": 0, "runtime": "claude-code" }),
            ),
            "build",
            2,
            1,
        ),
        of_execution(
            event(
                NOON + 3,
                "execution.failed",
                json!({ "code": "invalid_delivery", "message": "final JSON missing \"outputs\"" }),
            ),
            "build",
            2,
            1,
        ),
        in_step(
            event(
                NOON + 4,
                "step.blocked",
                json!({ "reason": "attempts_exhausted" }),
            ),
            "build",
            Some(2),
        ),
        in_step(
            event(NOON + 5, "step.unblocked", json!({ "budget_base": 3 })),
            "build",
            Some(2),
        ),
    ];
    let events = events
        .into_iter()
        .map(|event| of_task(event, "B"))
        .collect();

    assert_eq!(
        shown(&timeline(Some("B"), events)),
        "12:00:00  iteration.started  work · iter 2 until review not met: changes_requested\n\
         12:00:01  step.running       build iter 2\n\
         12:00:02  execution.started  build iter 2 · attempt 1 · agent-0 · claude-code · 0192f3a8-7b1c-7000-8000-000000000001\n\
         12:00:03  execution.failed   build iter 2 · attempt 1 · invalid_delivery — final JSON missing \"outputs\"\n\
         12:00:04  step.blocked       build iter 2 · attempts_exhausted\n\
         12:00:05  step.unblocked     build iter 2 · budget_base 3\n"
    );
}

#[test]
fn events_without_details_render_as_their_bare_kind() {
    let events = vec![
        event(NOON, "run.created", json!({})),
        event(NOON + 1, "driver.started", json!({})),
        event(
            NOON + 2,
            "approval.recorded",
            json!({ "decision": "approve" }),
        ),
        event(NOON + 3, "cleanup.finished", json!({})),
        event(NOON + 4, "agents.paused", json!({ "unexpected": true })),
    ];

    assert_eq!(
        shown(&timeline(None, events)),
        "12:00:00  run.created\n\
         12:00:01  driver.started\n\
         12:00:02  approval.recorded  approved\n\
         12:00:03  cleanup.finished\n\
         12:00:04  agents.paused\n"
    );
}

#[test]
fn events_from_an_earlier_day_carry_their_date() {
    let events = vec![
        event(NOON - 86_400, "driver.started", json!({ "pid": 1 })),
        event(NOON, "driver.stopped", json!({ "pid": 1 })),
    ];

    assert_eq!(
        shown(&timeline(None, events)),
        "2025-10-16 12:00:00  driver.started  pid 1\n\
         12:00:00             driver.stopped  pid 1\n"
    );
}

#[test]
fn recovery_stop_requests_intentions_and_cleanup_name_what_they_touched() {
    let events = vec![
        event(
            NOON,
            "driver.recovered",
            json!({ "previous_pid": 41, "found": [{ "kind": "orphan_stopped", "item": "A", "step": "build", "iteration": 1, "attempt": 1, "pid": 9 }] }),
        ),
        event(
            NOON + 1,
            "run.stop_requested",
            json!({ "actor": "you@example.com" }),
        ),
        event(
            NOON + 2,
            "intention.confirmed",
            json!({ "intention": "ref_move", "ref": "refs/heads/surt/x", "old_sha": "1111111aaaa", "new_sha": "2222222bbbb" }),
        ),
        event(
            NOON + 3,
            "cleanup.incomplete",
            json!({ "kept": [{ "kind": "unrecorded_edit", "path": "spec.md" }], "left": [{ "item": "worktree B", "reason": "busy" }] }),
        ),
    ];

    assert_eq!(
        shown(&timeline(None, events)),
        "12:00:00  driver.recovered     previous pid 41 · found orphan_stopped task A build\n\
         12:00:01  run.stop_requested   by you@example.com\n\
         12:00:02  intention.confirmed  ref_move refs/heads/surt/x 1111111 → 2222222\n\
         12:00:03  cleanup.incomplete   kept spec.md · left worktree B: busy\n"
    );
}

#[test]
fn a_timeline_prints_only_ascii_in_ascii_mode() {
    let events = vec![
        event(
            NOON,
            "approval.recorded",
            json!({ "decision": "reject", "artifact": "spec", "revision": 1, "actor": "you@example.com", "reason": "needs OAuth" }),
        ),
        event(
            NOON + 1,
            "intention.confirmed",
            json!({ "intention": "ref_move", "ref": "refs/heads/surt/x", "old_sha": "1111111aaaa", "new_sha": "2222222bbbb" }),
        ),
        of_execution(
            event(
                NOON + 2,
                "execution.failed",
                json!({ "code": "invalid_delivery", "message": "final JSON missing" }),
            ),
            "build",
            2,
            1,
        ),
    ];

    assert_eq!(
        Symbols::Ascii.render(&shown(&timeline(None, events))),
        "12:00:00  approval.recorded    spec rev 1 rejected - \"needs OAuth\" | by you@example.com\n\
         12:00:01  intention.confirmed  ref_move refs/heads/surt/x 1111111 -> 2222222\n\
         12:00:02  execution.failed     build iter 2 | attempt 1 | invalid_delivery - final JSON missing\n"
    );
}

#[test]
fn the_pause_shows_until_when_why_and_how_many_wait_and_the_resume_how_long_it_lasted() {
    let events = vec![
        event(
            NOON,
            "agents.paused",
            json!({ "until": NOON + 3_600, "code": "rate_limit", "waiting": 2 }),
        ),
        event(
            NOON + 60,
            "agents.paused",
            json!({ "until": NOON + 180, "code": "overloaded", "backoff_step": 2, "waiting": 1 }),
        ),
        event(
            NOON + 3_600,
            "agents.resumed",
            json!({ "paused_seconds": 3_600 }),
        ),
        event(
            NOON + 3_601,
            "agents.paused",
            json!({ "until": NOON + 86_400, "code": "rate_limit", "waiting": 1 }),
        ),
    ];

    assert_eq!(
        shown(&timeline(None, events)),
        "12:00:00  agents.paused   until 13:00 · rate_limit · 2 waiting\n\
         12:01:00  agents.paused   until 12:03 · overloaded · backoff step 2 · 1 waiting\n\
         13:00:00  agents.resumed  after 1h00m00s\n\
         13:00:01  agents.paused   until 2025-10-18 12:00 · rate_limit · 1 waiting\n"
    );
}
