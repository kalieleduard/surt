use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStderr};
use std::time::{Duration, Instant};

use jiff::Timestamp;
use serde_json::{Value, json};

use crate::scripts::{tasks, wait_until_stopped};
use crate::start::{feature_name, planned_with};
use crate::status::hashes;
use crate::support::Sandbox;

const CEILING: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(20);
const ONE_AGENT: &str = "max_concurrent_agents = 1\n";
const BUILD_STARTED: &str = "● task A build started\n";
const SQLITE_SHARED_MEMORY: &str = "surt.db-shm";

fn second_feature(sandbox: &Sandbox) -> String {
    let name = feature_name().replace("-add-login", "-fix-cache");
    for args in [
        vec!["feature", "fix cache"],
        vec!["approve", &name],
        vec!["plan", &name],
        vec!["approve", &name],
    ] {
        let run = sandbox.surt(&sandbox.repo(), &args);
        assert_eq!(run.code, Some(0), "surt {args:?}: {}", run.stderr);
    }
    name
}

struct Driver {
    child: Child,
    stderr: BufReader<ChildStderr>,
}

impl Driver {
    fn start(sandbox: &Sandbox, name: &str) -> Driver {
        let mut child = sandbox.spawn(&["start", name]);
        let stderr = BufReader::new(child.stderr.take().expect("the piped stderr"));
        Driver { child, stderr }
    }

    fn building(sandbox: &Sandbox, name: &str) -> Driver {
        let mut driver = Driver::start(sandbox, name);
        let mut seen = String::new();
        while !seen.ends_with(BUILD_STARTED) {
            let read = driver
                .stderr
                .read_line(&mut seen)
                .expect("stderr is readable");
            assert_ne!(read, 0, "stderr ended before the build started:\n{seen}");
        }
        driver
    }

    fn kill(mut self) {
        self.child.kill().expect("the driver is killed");
        self.child.wait().expect("the killed driver is reaped");
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn build_of(sandbox: &Sandbox, name: &str, column: &str) -> Vec<String> {
    sandbox.texts(&format!(
        "SELECT coalesce(CAST({column} AS TEXT), '') FROM step_run s \
         JOIN workflow_run w ON w.id = s.workflow_run_id JOIN subject u ON u.id = w.subject_id \
         LEFT JOIN execution e ON e.step_run_id = s.id \
         WHERE u.name = '{name}' AND w.item_key = 'A' AND s.step_key = 'build'"
    ))
}

fn running_agents(sandbox: &Sandbox) -> usize {
    sandbox
        .texts("SELECT id FROM execution WHERE kind = 'agent' AND state = 'running'")
        .len()
}

fn wait_until(sandbox: &Sandbox, what: &str, holds: impl Fn(&Sandbox) -> bool) {
    let deadline = Instant::now() + CEILING;
    while !holds(sandbox) {
        assert!(Instant::now() < deadline, "{what} never happened");
        std::thread::sleep(POLL);
    }
}

fn status(sandbox: &Sandbox, args: &[&str]) -> String {
    let run = sandbox.surt(&sandbox.repo(), args);
    assert_eq!(run.code, Some(0), "surt {args:?}: {}", run.stderr);
    run.stdout
}

fn agent_slots(document: &str) -> Vec<Value> {
    let document: Value = serde_json::from_str(document).expect("a JSON document");
    match document.get("features") {
        Some(Value::Array(features)) => features
            .iter()
            .map(|feature| feature["agent_slots"].clone())
            .collect(),
        _ => vec![document["agent_slots"].clone()],
    }
}

fn readable_files(sandbox: &Sandbox) -> BTreeMap<String, String> {
    let mut files = hashes(&sandbox.dot_surt());
    // Readers of a WAL database record read marks in its shared memory.
    files.remove(SQLITE_SHARED_MEMORY);
    files
}

#[test]
fn status_from_a_third_process_counts_the_agent_slots_of_both_driven_features() {
    let first = feature_name();
    let sandbox = planned_with(
        &first,
        &[tasks(1, &["A"]), wait_until_stopped("A/build")],
        Some(ONE_AGENT),
    );
    let second = second_feature(&sandbox);
    let _holder = Driver::building(&sandbox, &first);
    let _waiter = Driver::start(&sandbox, &second);
    let queued = "waiting for agent slot (1/1 in use)";
    wait_until(&sandbox, "the second Feature's Task queuing", |sandbox| {
        status(sandbox, &["status", &second]).contains(queued)
    });

    let before = readable_files(&sandbox);
    let overview = status(&sandbox, &["status"]);
    let overview_json = status(&sandbox, &["status", "--json"]);
    let holder = status(&sandbox, &["status", &first]);
    let waiter = status(&sandbox, &["status", &second]);
    let waiter_json = status(&sandbox, &["status", &second, "--json"]);
    let after = readable_files(&sandbox);

    assert!(overview.ends_with("agent slots: 1/1 in use\n"), "{overview}");
    let one_of_one = json!({ "in_use": 1, "total": 1 });
    assert_eq!(agent_slots(&overview_json), [one_of_one.clone(), one_of_one.clone()]);
    assert_eq!(agent_slots(&waiter_json), [one_of_one]);
    assert!(
        holder.lines().any(|line| line.starts_with("    A  ● build")),
        "{holder}"
    );
    assert!(
        waiter
            .lines()
            .any(|line| line.starts_with("    A  ") && line.ends_with(queued)),
        "{waiter}"
    );
    assert_eq!(after, before, "a status read wrote to .surt");
}

#[test]
fn two_drivers_of_one_project_never_run_more_agents_than_its_limit() {
    let first = feature_name();
    let sandbox = planned_with(
        &first,
        &[tasks(1, &["A"]), wait_until_stopped("A/build")],
        Some(ONE_AGENT),
    );
    let second = second_feature(&sandbox);
    let holder = Driver::building(&sandbox, &first);

    let waiter = Driver::start(&sandbox, &second);
    wait_until(
        &sandbox,
        "the second Feature's build becoming ready",
        |sandbox| {
            assert_eq!(running_agents(sandbox), 1, "agents of both Features ran");
            build_of(sandbox, &second, "s.state") == ["pending"]
        },
    );
    assert_eq!(running_agents(&sandbox), 1);
    assert_eq!(build_of(&sandbox, &second, "e.state"), [""]);
    holder.kill();
    let freed_at = Timestamp::now().as_second();
    wait_until(&sandbox, "the second Feature's build starting", |sandbox| {
        build_of(sandbox, &second, "e.state") == ["running"]
    });
    waiter.kill();

    let started: Vec<i64> = build_of(&sandbox, &second, "e.started_at")
        .iter()
        .map(|started| started.parse().expect("a timestamp"))
        .collect();
    assert!(started[0] >= freed_at, "{started:?} before {freed_at}");
    assert_eq!(build_of(&sandbox, &second, "e.slot"), ["0"]);
}
