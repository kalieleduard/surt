use jiff::Timestamp;
use serde_json::{Value, json};

use crate::delivering::subject_name;
use crate::inspect::{crashed_build, delivered_after_a_rejected_review};
use crate::status::{at_the_manifest_gate, delivered, with_a_blocked_item};
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const SCHEMA: u64 = 1;
const USAGE_ERROR_EXIT_CODE: i32 = 2;
const NOT_UTF8_BLOB: &str = "0000000000000000000000000000000000000000000000000000000000000001";

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn document(run: Run) -> Value {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    serde_json::from_str(&run.stdout).expect("one JSON document on stdout")
}

fn text(sandbox: &Sandbox, sql: &str) -> String {
    sandbox.texts(sql).concat()
}

fn rfc3339(sandbox: &Sandbox, sql: &str) -> String {
    let seconds: i64 = text(sandbox, sql).parse().expect("a unix timestamp");
    Timestamp::from_second(seconds)
        .expect("a timestamp")
        .to_string()
}

#[test]
fn status_json_of_a_blocked_subject_says_which_item_is_blocked_why_and_how_to_resume() {
    let name = subject_name();
    let sandbox = with_a_blocked_item(&name);
    let merged = text(
        &sandbox,
        "SELECT b.merged_as_sha FROM run_branch b JOIN workflow_run w ON w.id = b.workflow_run_id \
         WHERE w.item_key = 'A'",
    );
    let failed = text(
        &sandbox,
        "SELECT e.id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
         JOIN workflow_run w ON w.id = s.workflow_run_id \
         WHERE w.item_key = 'B' AND e.attempt = 3",
    );
    let spec_sha = text(
        &sandbox,
        "SELECT r.identity_sha256 FROM artifact_revision r JOIN artifact a ON a.id = r.artifact_id \
         WHERE a.name = 'spec'",
    );
    let spec_at = rfc3339(
        &sandbox,
        "SELECT CAST(min(decided_at) AS TEXT) FROM approval WHERE decision = 'approve'",
    );

    let status = document(surt(&sandbox, &["status", &name, "--json"]));

    assert_eq!(status["schema"], SCHEMA);
    assert_eq!(status["name"], name.as_str());
    assert_eq!(status["run"], 1);
    assert_eq!(status["state"], "BLOCKED");
    assert_eq!(status["driver"], Value::Null);
    assert_eq!(status["resume"], format!("surt continue {name}"));
    let spec = &status["gates"][1];
    assert_eq!(spec["artifact"], "spec");
    assert_eq!(spec["status"], "approved");
    assert_eq!(
        spec["revision"],
        json!({"number": 1, "origin": "agent", "sha256": spec_sha, "files": ["spec.md"]})
    );
    assert_eq!(
        spec["approval"],
        json!({"decision": "approve", "actor": "tester@example.com", "decided_at": spec_at})
    );
    let items = &status["items"];
    assert_eq!(items[0]["key"], "A");
    assert_eq!(items[0]["state"], "succeeded");
    assert_eq!(items[0]["merged_sha"], merged);
    assert_eq!(items[0].get("blocked_reason"), None);
    let blocked = &items[1];
    assert_eq!(blocked["key"], "B");
    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["step"], "build");
    assert_eq!(blocked["iteration"], 1);
    assert_eq!(blocked["attempt"], 3);
    assert_eq!(blocked["blocked_reason"], "attempts_exhausted");
    assert_eq!(
        blocked["last_error"],
        json!({"execution": failed, "attempt": 3, "code": "exit_code", "message": "the agent exited with code 1"})
    );
    assert_eq!(blocked["resume"], format!("surt continue {name}"));
    assert_utc(&blocked["since"]);
}

#[test]
fn status_json_lists_active_subjects_and_an_empty_list_when_none_is_active() {
    let name = subject_name();
    let sandbox = delivered(&name);

    let active = surt(&sandbox, &["status", "--json"]);
    let all = document(surt(&sandbox, &["status", "--all", "--json"]));

    assert!(active.stderr.is_empty(), "{}", active.stderr);
    assert_eq!(document(active), json!({"schema": 1, "subjects": []}));
    assert_eq!(all["schema"], SCHEMA);
    let subjects = all["subjects"].as_array().expect("a list of subjects");
    assert_eq!(subjects.len(), 1);
    assert_eq!(subjects[0]["name"], name.as_str());
    assert_eq!(subjects[0]["state"], "SUCCEEDED");
    assert_eq!(subjects[0]["states"], json!(["SUCCEEDED"]));
    assert_utc(&subjects[0]["ended_at"]);
}

#[test]
fn show_json_returns_the_revision_metadata_with_each_file_as_text_or_base64() {
    let name = subject_name();
    let sandbox = at_the_manifest_gate(&name);
    sandbox.execute(&format!(
        "INSERT INTO blob (sha256, size, bytes) VALUES ('{NOT_UTF8_BLOB}', 4, X'FFFE0041')"
    ));
    sandbox.execute(&format!(
        "UPDATE revision_file SET blob_sha256 = '{NOT_UTF8_BLOB}' WHERE rel_path = 'tasks/A.md'"
    ));
    let spec_sha = revision_sha(&sandbox, "spec", 1);
    let manifest_sha = revision_sha(&sandbox, "manifest", 1);
    let spec_at = rfc3339(
        &sandbox,
        "SELECT CAST(min(decided_at) AS TEXT) FROM approval WHERE decision = 'approve'",
    );

    let spec = document(surt(&sandbox, &["show", &name, "spec", "--json"]));
    let manifest = document(surt(&sandbox, &["show", &name, "manifest", "--json"]));
    let one = document(surt(
        &sandbox,
        &["show", &name, "manifest", "--file", "tasks/B.md", "--json"],
    ));

    assert_eq!(
        spec,
        json!({
            "schema": 1,
            "artifact": "spec",
            "revision": 1,
            "origin": "agent",
            "sha256": spec_sha,
            "approval": {"decision": "approve", "actor": "tester@example.com", "decided_at": spec_at},
            "files": [{"path": "spec.md", "content": "# Spec 1\n"}]
        })
    );
    assert_eq!(manifest["revision"], 1);
    assert_eq!(manifest["origin"], "agent");
    assert_eq!(manifest["sha256"], manifest_sha);
    assert_eq!(manifest.get("approval"), None);
    let files = manifest["files"].as_array().expect("a list of files");
    assert_eq!(files.len(), 3);
    assert_eq!(
        files[1],
        json!({"path": "tasks/A.md", "content_base64": "//4AQQ=="})
    );
    assert_eq!(files[2]["path"], "tasks/B.md");
    assert_eq!(files[2]["content"], "# Task B\n");
    assert_eq!(one["files"], json!([files[2]]));
}

#[test]
fn show_json_without_an_artifact_lists_the_artifacts() {
    let name = subject_name();
    let sandbox = at_the_manifest_gate(&name);

    let index = document(surt(&sandbox, &["show", &name, "--json"]));

    assert_eq!(index["schema"], SCHEMA);
    let spec = index["artifacts"]
        .as_array()
        .expect("a list of artifacts")
        .iter()
        .find(|artifact| artifact["name"] == "spec")
        .expect("the spec");
    assert_eq!(
        spec,
        &json!({"name": "spec", "revisions": 1, "latest": 1, "origin": "agent", "approved": true, "files": 1})
    );
}

#[test]
fn inspect_json_returns_the_timeline_events_with_their_scope_and_details() {
    let name = subject_name();
    let sandbox = delivered_after_a_rejected_review(&name);
    let crashed = crashed_build(&sandbox);

    let run = document(surt(&sandbox, &["inspect", &name, "--json"]));
    let item = document(surt(&sandbox, &["inspect", &name, "B", "--json"]));

    assert_eq!(run["schema"], SCHEMA);
    assert_eq!(run["subject"], name.as_str());
    assert_eq!(run["run"], 1);
    assert_eq!(run.get("item"), None);
    let events = run["events"].as_array().expect("a list of events");
    assert_eq!(events[0]["kind"], "run.created");
    for event in events {
        assert_utc(&event["at"]);
        assert!(event["data"].is_object(), "{event}");
    }
    let rejected = events
        .iter()
        .find(|event| event["kind"] == "approval.recorded")
        .expect("the rejection");
    assert_eq!(rejected["step"], "approve-spec");
    assert_eq!(item["item"], "B");
    let failed = item["events"]
        .as_array()
        .expect("a list of events")
        .iter()
        .find(|event| event["kind"] == "execution.failed")
        .expect("the crash of B");
    assert_eq!(failed["execution"], crashed.as_str());
    assert_eq!(failed["step"], "build");
    assert_eq!(failed["iteration"], 1);
    assert_eq!(failed["attempt"], 1);
    assert_eq!(failed["item"], "B");
}

#[test]
fn inspect_json_of_an_execution_returns_its_detail_without_unreported_values() {
    let name = subject_name();
    let sandbox = delivered_after_a_rejected_review(&name);
    let id = crashed_build(&sandbox);

    let detail = document(surt(&sandbox, &["inspect", &id, "--json"]));

    let execution = &detail["execution"];
    assert_eq!(detail["schema"], SCHEMA);
    assert_eq!(execution["id"], id.as_str());
    assert_eq!(execution["subject"], name.as_str());
    assert_eq!(execution["run"], 1);
    assert_eq!(execution["item"], "B");
    assert_eq!(execution["step"], "build");
    assert_eq!(execution["iteration"], 1);
    assert_eq!(execution["attempt"], 1);
    assert_eq!(execution["state"], "failed");
    assert_eq!(execution["failure"]["code"], "exit_code");
    assert_eq!(
        execution["performer"],
        json!({
            "kind": "agent",
            "role": "builder",
            "runtime": "claude-code",
            "model": "claude-sonnet-5-5",
            "effort": "high",
            "permission_mode": "dontAsk"
        })
    );
    assert_utc(&execution["started_at"]);
    assert_utc(&execution["finished_at"]);
    assert_eq!(execution["reported"].get("turns"), None);
    assert_eq!(execution["reported"].get("cost_usd"), None);
    assert_eq!(execution.get("salvage_ref"), None);
    let input = &execution["inputs"][0];
    assert_eq!(input["sha256"].as_str().map(str::len), Some(64));
}

#[test]
fn a_refusal_under_json_keeps_the_human_message_and_exit_code_and_prints_nothing_on_stdout() {
    let name = subject_name();
    let sandbox = at_the_manifest_gate(&name);

    for args in [
        vec!["status", "no-such-subject", "--json"],
        vec!["show", &name, "no-such-artifact", "--json"],
        vec!["inspect", &name, "Z", "--json"],
    ] {
        let human: Vec<&str> = args
            .iter()
            .copied()
            .filter(|arg| *arg != "--json")
            .collect();
        let refused = surt(&sandbox, &args);
        let expected = surt(&sandbox, &human);

        assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
        assert!(refused.stdout.is_empty(), "{}", refused.stdout);
        assert!(refused.stderr.starts_with('✖'), "{}", refused.stderr);
        assert_eq!(refused.stderr, expected.stderr);
    }
}

#[test]
fn driving_commands_reject_json_as_invalid_usage() {
    let name = subject_name();
    let sandbox = at_the_manifest_gate(&name);

    for command in [
        "continue", "approve", "reject", "cancel", "stop", "restore", "delete",
    ] {
        let run = surt(&sandbox, &[command, &name, "--json"]);

        assert_eq!(
            run.code,
            Some(USAGE_ERROR_EXIT_CODE),
            "{command}: {}",
            run.stderr
        );
        assert!(run.stdout.is_empty(), "{command}: {}", run.stdout);
    }
    let init = surt(&sandbox, &["init", "--json"]);
    assert_eq!(init.code, Some(USAGE_ERROR_EXIT_CODE), "{}", init.stderr);
}

fn revision_sha(sandbox: &Sandbox, artifact: &str, number: u32) -> String {
    text(
        sandbox,
        &format!(
            "SELECT r.identity_sha256 FROM artifact_revision r JOIN artifact a ON a.id = r.artifact_id \
             WHERE a.name = '{artifact}' AND r.number = {number}"
        ),
    )
}

fn assert_utc(value: &Value) {
    let text = value.as_str().expect("a timestamp string");
    assert!(text.ends_with('Z'), "{text}");
    text.parse::<Timestamp>().expect("an RFC 3339 timestamp");
}
