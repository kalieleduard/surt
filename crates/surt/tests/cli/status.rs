use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use sha2::{Digest, Sha256};

use crate::scripts::{build, crash_in, review, ship, spec, tasks};
use crate::start::{feature_name, planned};
use crate::support::{Run, Sandbox};

const BLOCKED_EXIT_CODE: i32 = 4;
const SHORT_SHA_CHARS: usize = 7;

const SQLITE_SHARED_MEMORY: &str = "surt.db-shm";
const SQLITE_WRITE_AHEAD_LOG: &str = "surt.db-wal";
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn hashes(root: &Path) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("a readable directory") {
            let path = entry.expect("a directory entry").path();
            let relative = path
                .strip_prefix(root)
                .expect("a path under the root")
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                found.insert(relative, "directory".to_owned());
                pending.push(path);
            } else {
                let bytes = fs::read(&path).expect("a readable file");
                let hex: String = Sha256::digest(&bytes)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                found.insert(relative, hex);
            }
        }
    }
    found
}

fn at_the_manifest_gate(name: &str) -> Sandbox {
    let sandbox = project(&[spec(1, "# Spec 1\n"), tasks(1, &["A", "B"])]);
    ok(sandbox.feature(&["add login"]));
    ok(surt(&sandbox, &["approve", name]));
    ok(surt(&sandbox, &["plan", name]));
    sandbox
}

fn git(sandbox: &Sandbox, args: &[&str]) -> String {
    sandbox.git_output(&sandbox.repo(), args).trim().to_owned()
}

fn local_time(sandbox: &Sandbox, sql: &str) -> String {
    let seconds: i64 = sandbox
        .texts(sql)
        .concat()
        .parse()
        .expect("a unix timestamp");
    Timestamp::from_second(seconds)
        .expect("a timestamp")
        .to_zoned(TimeZone::system())
        .strftime("%H:%M")
        .to_string()
}

#[test]
fn status_of_a_feature_at_a_gate_lists_every_covered_file_and_each_unrecorded_edit() {
    let name = feature_name();
    let sandbox = at_the_manifest_gate(&name);
    let working_copy = sandbox.dot_surt().join("features").join(&name);
    fs::write(working_copy.join("tasks/A.md"), "# Task A, edited\n").expect("an edit");
    fs::remove_file(working_copy.join("tasks/B.md")).expect("a removed file");
    let branch = git(&sandbox, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let head = git(&sandbox, &["rev-parse", "--short=7", "HEAD"]);
    let approved_at = local_time(&sandbox, "SELECT CAST(decided_at AS TEXT) FROM approval");

    let stdout = ok(surt(&sandbox, &["status", "add-login"]));

    assert_eq!(
        stdout,
        format!(
            "{name} · run 1   AWAITING APPROVAL   (no driver)\n  \
             base     {branch} @ {head}\n  \
             spec     rev 1 (agent) ✔ approved by tester@example.com {approved_at}\n  \
             manifest rev 1 (agent) ⏸ awaiting approval — approving covers:\n           \
             .surt/features/{name}/manifest.json\n           \
             .surt/features/{name}/tasks/A.md\n           \
             .surt/features/{name}/tasks/B.md\n           \
             ✎ tasks/A.md edited — recorded as rev 2 on approve (approval then targets rev 2)\n           \
             ⚠ tasks/B.md missing from the Working Copy\n  \
             approve: surt approve {name}    reject: surt reject {name} -m \"…\"\n"
        )
    );
}

fn short(sha: &str) -> &str {
    &sha[..SHORT_SHA_CHARS]
}

fn with_a_blocked_task(name: &str) -> Sandbox {
    let sandbox = planned(
        name,
        &[
            tasks(1, &["A", "B"]),
            build("A", 1, 1, "login.txt", "login\n"),
            review("A", 1, "approved"),
            crash_in("B/build", 1, 1),
            crash_in("B/build", 1, 2),
            crash_in("B/build", 1, 3),
        ],
    );
    let run = surt(&sandbox, &["start", name]);
    assert_eq!(run.code, Some(BLOCKED_EXIT_CODE), "{}", run.stderr);
    sandbox
}

#[test]
fn status_of_a_blocked_feature_shows_each_task_and_how_to_resume_each_blocked_step() {
    let name = feature_name();
    let sandbox = with_a_blocked_task(&name);
    let branch = git(&sandbox, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let head = git(&sandbox, &["rev-parse", "--short=7", "HEAD"]);
    let tip = git(
        &sandbox,
        &["rev-parse", "--short=7", &format!("surt/{name}")],
    );
    let merged = sandbox
        .texts(
            "SELECT b.merged_as_sha FROM run_branch b JOIN workflow_run w ON w.id = b.workflow_run_id \
             WHERE w.item_key = 'A'",
        )
        .concat();
    let execution = sandbox
        .texts(
            "SELECT e.id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id WHERE w.item_key = 'B' AND e.attempt = 3",
        )
        .concat();
    let spec_at = local_time(
        &sandbox,
        "SELECT CAST(min(decided_at) AS TEXT) FROM approval WHERE decision = 'approve'",
    );
    let manifest_at = local_time(
        &sandbox,
        "SELECT CAST(max(decided_at) AS TEXT) FROM approval WHERE decision = 'approve'",
    );

    let overview = ok(surt(&sandbox, &["status"]));
    let detail = ok(surt(&sandbox, &["status", &name]));

    assert_eq!(
        overview,
        format!(
            "FEATURE             RUN  STATE    DRIVER  PROGRESS\n\
             {name}  1    BLOCKED  —       1/2 merged · 1 blocked\n"
        )
    );
    assert_eq!(
        detail,
        format!(
            "{name} · run 1   BLOCKED   (no driver)\n  \
             base     {branch} @ {head}\n  \
             branch   surt/{name} @ {tip}\n  \
             spec     rev 1 (agent) ✔ approved by tester@example.com {spec_at}\n  \
             manifest rev 1 (agent) ✔ approved by tester@example.com {manifest_at}\n  \
             tasks\n    \
             A  ✔ merged    iter 1 · as {}\n    \
             B  ⛔ blocked   build iter 1 · attempt 3\n  \
             ⛔ task B build blocked: attempts exhausted\n     \
             attempt 3 failed (exit_code): the agent exited with code 1\n     \
             log: .surt/logs/{name}/{execution}/stream.jsonl\n  \
             resume: surt start {name}\n",
            short(&merged)
        )
    );
}

fn delivered(name: &str) -> Sandbox {
    let sandbox = planned(
        name,
        &[
            tasks(1, &["A"]),
            build("A", 1, 1, "login.txt", "login\n"),
            review("A", 1, "approved"),
            ship(),
        ],
    );
    ok(surt(&sandbox, &["start", name]));
    sandbox
}

#[test]
fn status_all_adds_terminal_runs_and_shows_a_pending_cleanup_with_how_to_retry_it() {
    let name = feature_name();
    let sandbox = delivered(&name);
    let execution = sandbox
        .texts("SELECT id FROM execution ORDER BY id LIMIT 1")
        .concat();
    let blocker = sandbox
        .dot_surt()
        .join("runtime/executions")
        .join(&execution);

    let active = surt(&sandbox, &["status"]);
    let cleaned = ok(surt(&sandbox, &["status", "--all"]));
    fs::write(&blocker, "not a directory\n").expect("a file in the way");
    sandbox.execute("UPDATE workflow_run SET cleaned_at = NULL");
    ok(surt(&sandbox, &["start", &name]));
    let pending = ok(surt(&sandbox, &["status", "--all"]));
    let detail = ok(surt(&sandbox, &["status", &name]));

    assert_eq!(active.code, Some(0), "{}", active.stderr);
    assert!(active.stdout.is_empty(), "{}", active.stdout);
    assert_eq!(
        active.stderr,
        "no feature has an active run (surt status --all lists finished ones)\n"
    );
    assert_eq!(
        cleaned,
        format!(
            "FEATURE             RUN  STATE      DRIVER  PROGRESS\n\
             {name}  1    SUCCEEDED  —       1/1 merged\n"
        )
    );
    assert_eq!(
        pending,
        format!(
            "FEATURE             RUN  STATE      DRIVER  PROGRESS\n\
             {name}  1    SUCCEEDED  —       1/1 merged · cleanup pending\n"
        )
    );
    assert!(
        detail.starts_with(&format!("{name} · run 1   SUCCEEDED\n")),
        "{detail}"
    );
    assert!(
        detail.contains(&format!(
            "\n  cleanup  pending\n           could not remove .surt/runtime/executions/{execution} ("
        )),
        "{detail}"
    );
    assert!(
        detail.ends_with(&format!(")\n           retry: surt start {name}\n")),
        "{detail}"
    );
}

#[test]
fn show_reads_any_revision_after_cleanup_including_a_tasks_own_artifacts() {
    let name = feature_name();
    let sandbox = delivered(&name);

    let spec = ok(surt(&sandbox, &["show", &name, "spec"]));
    let feedback = ok(surt(&sandbox, &["show", &name, "feedback", "--task", "A"]));
    let detail = ok(surt(&sandbox, &["status", &name]));

    assert!(!sandbox.dot_surt().join("features").join(&name).exists());
    assert_eq!(spec, "# Spec 1\n");
    assert!(
        feedback.contains("\"summary\":\"approved after reading the code\""),
        "{feedback}"
    );
    assert!(detail.ends_with("\n  cleanup  done\n"), "{detail}");
}

#[test]
fn status_and_show_write_nothing_to_the_database_or_the_working_copy() {
    let name = feature_name();
    let sandbox = at_the_manifest_gate(&name);
    let working_copy = sandbox.dot_surt().join("features").join(&name);
    fs::write(working_copy.join("tasks/A.md"), "# Task A, edited\n").expect("an edit");
    fs::remove_file(working_copy.join("tasks/B.md")).expect("a removed file");
    let before = hashes(&sandbox.dot_surt());

    for args in [
        vec!["status"],
        vec!["status", "--all"],
        vec!["status", &name],
        vec!["show", &name],
        vec!["show", &name, "manifest"],
        vec!["show", &name, "spec", "--rev", "1"],
    ] {
        ok(surt(&sandbox, &args));
    }

    let mut after = hashes(&sandbox.dot_surt());
    let shared_memory = after.remove(SQLITE_SHARED_MEMORY);
    let log = after.remove(SQLITE_WRITE_AHEAD_LOG);
    assert_eq!(after, before);
    assert!(shared_memory.is_none() || log.as_deref() == Some(EMPTY_SHA256));
    assert!(!working_copy.join("tasks/B.md").exists());
}

fn project(scripts: &[String]) -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(scripts);
    sandbox
}

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stdout
}

#[test]
fn status_lists_a_feature_awaiting_approval_with_no_driver_and_its_gate() {
    let name = feature_name();
    let sandbox = project(&[spec(1, "# Spec 1\n")]);
    ok(sandbox.feature(&["add login"]));

    let stdout = ok(surt(&sandbox, &["status"]));

    assert_eq!(
        stdout,
        format!(
            "FEATURE             RUN  STATE              DRIVER  PROGRESS\n\
             {name}  1    AWAITING APPROVAL  —       spec rev 1\n"
        )
    );
}
