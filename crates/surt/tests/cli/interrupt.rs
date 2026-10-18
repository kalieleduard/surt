use std::io::{BufRead, BufReader, Read};
use std::process::{Child, ChildStderr, Command};
use std::time::{Duration, Instant};

use crate::scripts::{build, tasks, wait_until_stopped};
use crate::start::{feature_name, planned};
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
        while !self.seen.ends_with(line) {
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

#[test]
fn a_second_ctrl_c_forces_the_stop_without_waiting_for_a_running_check() {
    let name = feature_name();
    let sandbox = planned(&name, &[tasks(1, &["A"]), build("A", 1, 1, "a.txt", "a\n")]);
    sandbox.write_config(SLOW_CHECK);
    let clock = Instant::now();
    let mut driving = Driving::start(&sandbox, &name);
    driving.wait_for("● task A check started\n");

    driving.signal("INT");
    driving.wait_for(STOPPING);
    driving.signal("INT");
    let (code, stderr) = driving.finish();

    assert_eq!(code, Some(INTERRUPTED_EXIT_CODE), "{stderr}");
    assert!(stderr.ends_with(&interrupted(&name)), "{stderr}");
    assert!(!stderr.contains("task A check passed"), "{stderr}");
    assert!(clock.elapsed() < FORCE_CEILING, "{:?}", clock.elapsed());
}
