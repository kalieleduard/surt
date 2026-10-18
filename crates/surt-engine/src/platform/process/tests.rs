use std::io::{BufRead, BufReader};
use std::process::{ChildStdin, Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::{ProcessIdentity, identify, is_alive, stop};

const SHORT_GRACE: Duration = Duration::from_millis(200);
const REAP_TIMEOUT: Duration = Duration::from_secs(10);
const REAP_POLL: Duration = Duration::from_millis(10);

struct Sleeper {
    identity: ProcessIdentity,
    reaped: JoinHandle<()>,
    _stdin: Option<ChildStdin>,
}

fn sleeper(mut command: Command) -> Sleeper {
    command.stdin(Stdio::piped()).stderr(Stdio::null());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command.spawn().expect("the sleeper starts");
    if let Some(stdout) = child.stdout.take() {
        let mut ready = String::new();
        BufReader::new(stdout)
            .read_line(&mut ready)
            .expect("the sleeper says it is ready");
    }
    let identity = identify(child.id()).expect("a live child is identified");
    let stdin = child.stdin.take();
    let reaped = thread::spawn(move || {
        let _ = child.wait();
    });
    Sleeper {
        identity,
        reaped,
        _stdin: stdin,
    }
}

fn hashing_stdin() -> Command {
    let mut command = Command::new("git");
    command
        .args(["hash-object", "--stdin"])
        .current_dir(std::env::temp_dir())
        .stdout(Stdio::null());
    command
}

fn reaped(sleeper: Sleeper) {
    let deadline = Instant::now() + REAP_TIMEOUT;
    while !sleeper.reaped.is_finished() {
        assert!(Instant::now() < deadline, "the sleeper outlived its stop");
        thread::sleep(REAP_POLL);
    }
}

#[test]
fn the_current_process_has_a_stable_identity_with_a_start_time_and_a_boot_id() {
    let first = identify(std::process::id()).expect("this process exists");
    let again = identify(std::process::id()).expect("this process still exists");

    assert_eq!(first, again);
    assert_eq!(first.pid, std::process::id());
    assert!(!first.boot_id.is_empty());
    assert!(is_alive(&first));
}

#[test]
fn a_reused_pid_with_another_start_time_or_boot_is_not_the_recorded_process() {
    let current = identify(std::process::id()).expect("this process exists");
    let started_later = ProcessIdentity {
        start_time: current.start_time + 1,
        ..current.clone()
    };
    let previous_boot = ProcessIdentity {
        boot_id: format!("{}-before-a-reboot", current.boot_id),
        ..current
    };

    assert!(!is_alive(&started_later));
    assert!(!is_alive(&previous_boot));
}

#[tokio::test]
async fn a_live_process_is_stopped_and_an_exited_one_is_left_alone() {
    let sleeper = sleeper(hashing_stdin());
    let identity = sleeper.identity.clone();

    assert!(is_alive(&identity));
    assert!(stop(&identity, SHORT_GRACE).await);
    reaped(sleeper);
    assert!(!is_alive(&identity));
    assert!(!stop(&identity, SHORT_GRACE).await);
}

#[cfg(unix)]
#[tokio::test]
async fn a_process_group_ignoring_the_graceful_signal_is_killed_after_the_grace_period() {
    let mut command = Command::new("sh");
    command
        .args(["-c", "trap '' TERM; echo ready; read line"])
        .stdout(Stdio::piped());
    let sleeper = sleeper(command);
    let identity = sleeper.identity.clone();
    let started = Instant::now();

    assert!(stop(&identity, SHORT_GRACE).await);
    reaped(sleeper);
    assert!(started.elapsed() >= SHORT_GRACE);
    assert!(!is_alive(&identity));
}
