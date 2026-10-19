use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use crate::interrupt::Driving;
use crate::logs::{latest, logs_of};
use crate::scripts::{build, review, ship, spec, tasks_needing};
use crate::start::feature_name;
use crate::support::{Run, Sandbox};

const CEILING: Duration = Duration::from_secs(60);
const CHECK_STARTED: &str = "● task A check started\n";
const WAITING: &str = "waiting\n";

struct Following {
    child: Child,
    stderr: ChildStderr,
    lines: Receiver<String>,
    seen: String,
}

impl Following {
    fn start(sandbox: &Sandbox, args: &[&str]) -> Following {
        let mut child = sandbox.spawn(args);
        let mut stdout = BufReader::new(child.stdout.take().expect("the piped stdout"));
        let stderr = child.stderr.take().expect("the piped stderr");
        let (sender, lines) = mpsc::channel();
        thread::spawn(move || {
            let mut line = String::new();
            while stdout.read_line(&mut line).is_ok_and(|read| read > 0) {
                if sender.send(std::mem::take(&mut line)).is_err() {
                    break;
                }
            }
        });
        Following {
            child,
            stderr,
            lines,
            seen: String::new(),
        }
    }

    fn wait_for(&mut self, text: &str) {
        let deadline = Instant::now() + CEILING;
        while !self.seen.contains(text) {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) => self.seen.push_str(&line),
                Err(error) => {
                    let _ = self.child.kill();
                    let mut stderr = String::new();
                    let _ = self.stderr.read_to_string(&mut stderr);
                    panic!("{error:?} before {text:?}:\n{}\n{stderr}", self.seen);
                }
            }
        }
    }

    fn finish(mut self) -> Run {
        let deadline = Instant::now() + CEILING;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) => self.seen.push_str(&line),
                Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {
                    let _ = self.child.kill();
                    panic!("logs -f still running after {CEILING:?}:\n{}", self.seen);
                }
            }
        }
        let status = self.child.wait().expect("surt logs exits");
        let mut stderr = String::new();
        self.stderr
            .read_to_string(&mut stderr)
            .expect("stderr is readable");
        Run {
            code: status.code(),
            stdout: self.seen,
            stderr,
        }
    }
}

struct Gated {
    name: String,
    sandbox: Sandbox,
    gate: PathBuf,
}

impl Gated {
    fn new() -> Gated {
        let name = feature_name();
        let sandbox = Sandbox::with_repository();
        sandbox.commit_something();
        sandbox.init();
        let gate = sandbox.home().join("gate");
        sandbox.write_config(&format!(
            "[[checks]]\nname = \"gate\"\nrun = [\"sh\", \"-c\", {:?}]\n",
            script(&gate)
        ));
        sandbox.script(&[
            spec(1, "# Spec 1\n"),
            tasks_needing(1, &[("A", &[]), ("B", &["A"])]),
            build("A", 1, 1, "a.txt", "a\n"),
            review("A", 1, "approved"),
            build("B", 1, 1, "b.txt", "b\n"),
            review("B", 1, "approved"),
            ship(),
        ]);
        ok(sandbox.feature(&["add login"]));
        for command in ["approve", "plan", "approve"] {
            ok(sandbox.surt(&sandbox.repo(), &[command, &name]));
        }
        Gated {
            name,
            sandbox,
            gate,
        }
    }

    fn checked(&self) -> Vec<String> {
        vec![
            format!("=== gate: sh -c {}", script(&self.gate)),
            "waiting".to_owned(),
            "released".to_owned(),
        ]
    }

    fn release(&self, driving: Driving) {
        fs::write(&self.gate, "").expect("the gate opens");
        let (code, stderr) = driving.finish();
        assert_eq!(code, Some(0), "{stderr}");
    }
}

fn script(gate: &Path) -> String {
    format!(
        "echo waiting; while [ ! -e '{}' ]; do sleep 0.05; done; echo released",
        gate.display()
    )
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stdout
}

fn quiet(run: Run) -> String {
    assert_eq!(run.stderr, "");
    ok(run)
}

fn lines_of(stdout: &str, label: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| line.split_once(" │ "))
        .filter(|(prefix, _)| prefix.trim_end() == label)
        .map(|(_, text)| text.to_owned())
        .collect()
}

fn sorted_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    lines.sort();
    lines
}

fn streams_from_a_s_check_on(gated: &Gated) -> Vec<String> {
    let executions = gated.sandbox.texts(
        "SELECT x.id FROM execution x JOIN step_run s ON s.id = x.step_run_id \
         WHERE x.kind = 'agent' AND (s.step_key = 'plan-tasks' OR x.rowid > \
         (SELECT c.rowid FROM execution c JOIN step_run cs ON cs.id = c.step_run_id \
          JOIN workflow_run cw ON cw.id = cs.workflow_run_id \
          WHERE cw.item_key = 'A' AND cs.step_key = 'check')) ORDER BY x.rowid",
    );
    let streams: String = executions
        .iter()
        .map(|execution| {
            let stream = logs_of(&gated.sandbox, &gated.name, execution).join("stream.jsonl");
            fs::read_to_string(stream).expect("a stream")
        })
        .collect();
    sorted_lines(&streams)
}

#[test]
fn logs_f_follows_two_tasks_of_a_live_run_to_the_end() {
    let gated = Gated::new();
    let name = gated.name.as_str();
    let mut driving = Driving::start(&gated.sandbox, name);
    driving.wait_for(CHECK_STARTED);
    let mut rendered = Following::start(&gated.sandbox, &["logs", name, "-f"]);
    rendered.wait_for(WAITING);
    let mut raw = Following::start(&gated.sandbox, &["logs", name, "-f", "--raw"]);
    raw.wait_for("\n");

    gated.release(driving);
    let rendered = quiet(rendered.finish());
    let raw = quiet(raw.finish());

    assert_eq!(
        lines_of(&rendered, "A check#1"),
        gated.checked(),
        "{rendered}"
    );
    assert_eq!(
        lines_of(&rendered, "B check#1"),
        gated.checked(),
        "{rendered}"
    );
    assert_eq!(sorted_lines(&raw), streams_from_a_s_check_on(&gated));
}

#[test]
fn logs_f_of_a_task_or_an_execution_follows_it_to_the_end() {
    let gated = Gated::new();
    let name = gated.name.as_str();
    let mut driving = Driving::start(&gated.sandbox, name);
    driving.wait_for(CHECK_STARTED);
    let mut task = Following::start(&gated.sandbox, &["logs", name, "A", "-f"]);
    task.wait_for(WAITING);
    let check = latest(&gated.sandbox, "'A'", "check");
    let mut execution = Following::start(&gated.sandbox, &["logs", &check, "-f"]);
    execution.wait_for(WAITING);

    gated.release(driving);
    let task = quiet(task.finish());
    let execution = quiet(execution.finish());

    assert_eq!(lines_of(&task, "A check#1"), gated.checked(), "{task}");
    assert_eq!(execution, format!("{}\n", gated.checked().join("\n")));
}
