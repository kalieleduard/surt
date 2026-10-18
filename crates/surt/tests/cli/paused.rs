use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Child;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::scripts::{limit_failure_in, tasks};
use crate::start::{feature_name, planned};
use crate::support::{Run, Sandbox};

const DISTANT_RESET_SECONDS: i64 = 3_600;
const PAUSED: &str = "⏸ agent launches paused until ";
const DATABASE_FILES: [&str; 3] = ["surt.db", "surt.db-shm", "surt.db-wal"];
const LOCKED: &str = "locked";

struct PausedDriver {
    child: Child,
}

impl PausedDriver {
    fn start(sandbox: &Sandbox, name: &str) -> PausedDriver {
        let mut child = sandbox.spawn(&["start", name]);
        let mut stderr = BufReader::new(child.stderr.take().expect("the piped stderr"));
        let mut seen = String::new();
        while !seen.lines().any(|line| line.starts_with(PAUSED)) {
            let read = stderr.read_line(&mut seen).expect("stderr is readable");
            assert_ne!(read, 0, "stderr ended before launches paused:\n{seen}");
        }
        PausedDriver { child }
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for PausedDriver {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn fingerprint(sandbox: &Sandbox) -> (BTreeMap<String, String>, Vec<String>) {
    let mut files = BTreeMap::new();
    walk(&sandbox.dot_surt(), &sandbox.dot_surt(), &mut files);
    files.retain(|path, _| !DATABASE_FILES.contains(&path.as_str()));
    let rows = sandbox.texts(
        "SELECT 'event ' || count(*) || ' ' || coalesce(max(seq), 0) FROM event \
         UNION ALL SELECT 'run ' || id || ' ' || state || ' ' || coalesce(driver_pid, '') \
           || ' ' || coalesce(stop_requested_by, '') FROM workflow_run \
         UNION ALL SELECT 'step ' || id || ' ' || state FROM step_run \
         UNION ALL SELECT 'execution ' || id || ' ' || state FROM execution",
    );
    (files, rows)
}

fn walk(root: &Path, directory: &Path, found: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(directory).expect("a readable directory") {
        let path = entry.expect("a directory entry").path();
        let relative = path
            .strip_prefix(root)
            .expect("a path under the root")
            .to_string_lossy()
            .into_owned();
        if path.is_dir() {
            walk(root, &path, found);
            found.insert(relative, "directory".to_owned());
            continue;
        }
        let hash = fs::read(&path).map_or_else(
            |_| LOCKED.to_owned(),
            |bytes| {
                Sha256::digest(&bytes)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect()
            },
        );
        found.insert(relative, hash);
    }
}
fn resets_at(sandbox: &Sandbox) -> i64 {
    sandbox.texts(
        "SELECT CAST(json_extract(runtime_info, '$.resets_at') AS TEXT) FROM execution \
         WHERE failure_class = 'transient'",
    )[0]
    .parse()
    .expect("the reset time the failure recorded")
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stdout
}

#[test]
fn status_from_another_terminal_shows_a_paused_driver_and_writes_nothing() {
    let name = feature_name();
    let sandbox = planned(
        &name,
        &[
            tasks(1, &["A"]),
            limit_failure_in("A/build", DISTANT_RESET_SECONDS),
        ],
    );
    let driver = PausedDriver::start(&sandbox, &name);
    let before = fingerprint(&sandbox);

    let detail = ok(sandbox.surt(&sandbox.repo(), &["status", &name]));
    let document = ok(sandbox.surt(&sandbox.repo(), &["status", &name, "--json"]));

    assert_eq!(fingerprint(&sandbox), before);
    let pid = driver.pid();
    drop(driver);
    let resets_at = resets_at(&sandbox);
    let resets = Timestamp::from_second(resets_at)
        .expect("a timestamp")
        .to_zoned(TimeZone::system())
        .strftime("%H:%M")
        .to_string();
    assert!(
        detail.starts_with(&format!(
            "{name} · run 1   RUNNING   driver pid {pid} since "
        )),
        "{detail}"
    );
    assert!(
        detail.contains(&format!(
            "\n  ⏸ agent launches paused: rate_limit — resets {resets}\n"
        )),
        "{detail}"
    );
    assert!(
        detail.contains(
            "\n    A  ⏳ queued    build iter 1 · attempt 1 · waiting for agent launches to resume\n"
        ),
        "{detail}"
    );
    let document: Value = serde_json::from_str(&document).expect("a JSON document");
    assert_eq!(document["state"], json!("RUNNING"));
    assert_eq!(
        document["paused"],
        json!({
            "code": "rate_limit",
            "resets_at": Timestamp::from_second(resets_at).expect("a timestamp").to_string(),
        })
    );
}
