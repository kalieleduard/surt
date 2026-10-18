use std::io::{BufRead, BufReader, Read};
use std::process::{Child, ChildStderr, Command};
use std::time::{Duration, Instant};

use crate::scripts::{build, tasks, tasks_needing, wait_until_stopped};
use crate::start::{feature_name, planned, planned_with};
use crate::support::Sandbox;

const INTERRUPTED_EXIT_CODE: i32 = 130;
const FORCE_CEILING: Duration = Duration::from_secs(20);
const STOPPING: &str = "⏹ stopping — press Ctrl-C again to force\n";
const SLOW_CHECK: &str = "[[checks]]\nname = \"slow\"\nrun = [\"sleep\", \"60\"]\n";

pub struct Driving {
    child: Child,
    stderr: BufReader<ChildStderr>,
    seen: String,
}

impl Driving {
    pub fn start(sandbox: &Sandbox, name: &str) -> Driving {
        let mut child = sandbox.spawn(&["start", name]);
        let stderr = BufReader::new(child.stderr.take().expect("the piped stderr"));
        Driving {
            child,
            stderr,
            seen: String::new(),
        }
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn wait_for(&mut self, line: &str) {
        while !self.seen.contains(line) {
            let read = self
                .stderr
                .read_line(&mut self.seen)
                .expect("stderr is readable");
            assert_ne!(read, 0, "stderr ended before {line:?}:\n{}", self.seen);
        }
    }

    pub fn signal(&self, name: &str) {
        let status = Command::new("kill")
            .args([&format!("-{name}"), &self.child.id().to_string()])
            .status()
            .expect("kill runs");
        assert!(status.success(), "kill -{name} failed");
    }

    pub fn finish(mut self) -> (Option<i32>, String) {
        self.stderr
            .read_to_string(&mut self.seen)
            .expect("stderr is readable");
        let status = self.child.wait().expect("surt exits");
        (status.code(), self.seen)
    }
}

fn interrupted(name: &str) -> String {
    format!("⏹ interrupted\n  resume: surt start {name}\n")
}

pub fn waiting_builder(name: &str) -> Sandbox {
    planned(name, &[tasks(1, &["A"]), wait_until_stopped("A/build")])
}

fn interrupts_with(signal: &str) {
    let name = feature_name();
    let sandbox = waiting_builder(&name);
    let mut driving = Driving::start(&sandbox, &name);
    driving.wait_for("● task A build started\n");

    driving.signal(signal);
    let (code, stderr) = driving.finish();

    assert_eq!(code, Some(INTERRUPTED_EXIT_CODE), "{stderr}");
    assert!(stderr.contains(STOPPING), "{stderr}");
    assert!(
        stderr.contains("⏹ task A build attempt 1 interrupted\n"),
        "{stderr}"
    );
    assert!(stderr.ends_with(&interrupted(&name)), "{stderr}");
    assert_eq!(
        sandbox
            .texts("SELECT state FROM execution WHERE agent_snapshot_sha256 IS NOT NULL ORDER BY rowid DESC LIMIT 1")
            .concat(),
        "interrupted"
    );
}

#[test]
fn ctrl_c_stops_the_builders_gracefully_and_exits_130() {
    interrupts_with("INT");
}

#[test]
fn a_closed_terminal_stops_the_builders_gracefully_and_exits_130() {
    interrupts_with("HUP");
}

const SPAWNED: &str = "SELECT CAST(count(*) AS TEXT) FROM execution WHERE pid IS NOT NULL";

fn wait_until_spawned(sandbox: &Sandbox) {
    let deadline = Instant::now() + FORCE_CEILING;
    while sandbox.texts(SPAWNED).concat() == "0" {
        assert!(Instant::now() < deadline, "the slow Check never spawned");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_second_ctrl_c_forces_the_stop_without_waiting_for_a_running_check() {
    let name = feature_name();
    let sandbox = planned_with(
        &name,
        &[tasks(1, &["A"]), build("A", 1, 1, "a.txt", "a\n")],
        Some(SLOW_CHECK),
    );
    let clock = Instant::now();
    let mut driving = Driving::start(&sandbox, &name);
    driving.wait_for("● task A check started\n");
    wait_until_spawned(&sandbox);

    driving.signal("INT");
    driving.wait_for(STOPPING);
    driving.signal("INT");
    let (code, stderr) = driving.finish();

    assert_eq!(code, Some(INTERRUPTED_EXIT_CODE), "{stderr}");
    assert!(stderr.ends_with(&interrupted(&name)), "{stderr}");
    assert!(!stderr.contains("task A check passed"), "{stderr}");
    assert_eq!(
        sandbox
            .texts(
                "SELECT state FROM execution WHERE pid IS NOT NULL ORDER BY rowid DESC LIMIT 1"
            )
            .concat(),
        "running",
        "the forced stop leaves the slow Check to recovery"
    );
    assert!(clock.elapsed() < FORCE_CEILING, "{:?}", clock.elapsed());
}

#[test]
fn status_shows_a_driven_feature_as_running_with_its_driver_and_running_task() {
    let name = feature_name();
    let sandbox = waiting_builder(&name);
    let mut driving = Driving::start(&sandbox, &name);
    driving.wait_for("● task A build started\n");
    let pid = driving.child.id();

    let overview = sandbox.surt(&sandbox.repo(), &["status"]);
    let detail = sandbox.surt(&sandbox.repo(), &["status", &name]);
    driving.signal("INT");
    let (code, stderr) = driving.finish();

    assert_eq!(code, Some(INTERRUPTED_EXIT_CODE), "{stderr}");
    assert_eq!(
        cells(&overview.stdout),
        [
            vec!["FEATURE", "RUN", "STATE", "DRIVER", "PROGRESS"],
            vec![
                name.as_str(),
                "1",
                "RUNNING",
                &format!("pid {pid}"),
                "0/1 merged · 1 running"
            ],
            vec!["agent slots: 1/3 in use"],
        ],
        "{}",
        overview.stderr
    );
    assert!(
        detail.stdout.starts_with(&format!(
            "{name} · run 1   RUNNING   driver pid {pid} since "
        )),
        "{}",
        detail.stdout
    );
    assert!(
        detail
            .stdout
            .contains("\n  implement\n    A  ● build     iter 1 · attempt 1 · since "),
        "{}",
        detail.stdout
    );
}

fn cells(table: &str) -> Vec<Vec<&str>> {
    table
        .lines()
        .map(|line| {
            line.split("  ")
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect()
        })
        .collect()
}

#[test]
fn status_shows_a_task_queued_for_an_agent_slot_and_one_waiting_for_its_needs() {
    let name = feature_name();
    let sandbox = planned(
        &name,
        &[
            tasks_needing(1, &[("A", &[]), ("B", &[]), ("C", &["A"])]),
            wait_until_stopped("A/build"),
        ],
    );
    sandbox.write_config("max_concurrent_agents = 1\n");
    let mut driving = Driving::start(&sandbox, &name);
    driving.wait_for("● task A build started\n");
    driving.wait_for("✔ task B setup done\n");

    let overview = sandbox.surt(&sandbox.repo(), &["status"]);
    let detail = sandbox.surt(&sandbox.repo(), &["status", &name]);
    driving.signal("INT");
    let (code, stderr) = driving.finish();

    assert_eq!(code, Some(INTERRUPTED_EXIT_CODE), "{stderr}");
    assert!(
        overview
            .stdout
            .ends_with("0/3 merged · 1 running · 1 queued\nagent slots: 1/1 in use\n"),
        "{}{}",
        overview.stdout,
        detail.stdout
    );
    for line in [
        "\n  implement\n    A  ● build     iter 1 · attempt 1 · since ",
        "\n    B  ⏳ queued    ",
        "waiting for agent slot (1/1 in use)\n",
        "\n    C  ⏸ pending   waits for A\n",
    ] {
        assert!(detail.stdout.contains(line), "{line}\n{}", detail.stdout);
    }
}
