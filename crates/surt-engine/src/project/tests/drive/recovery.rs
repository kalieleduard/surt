use std::fs;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tokio::sync::{Notify, mpsc};

use super::fixtures::{Feature, draft, label};
use crate::project::{Cancellation, DriveOutcome, Progress, RunState, StatusScope, Unattended};
use crate::runtime::{
    AgentRuntime, Behavior, FakeRuntime, RuntimeCompletion, RuntimeError, RuntimeEvent,
    RuntimeEventSink, RuntimeRequest, Script, StopSignal,
};

const RECORDING_TIMEOUT: Duration = Duration::from_secs(10);
const RECORDING_POLL: Duration = Duration::from_millis(10);
const EXIT_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) struct Started {
    pub(super) fake: FakeRuntime,
    pub(super) launched: Notify,
}

impl AgentRuntime for Started {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        self.launched.notify_one();
        self.fake.execute(request, events, stop).await
    }
}

pub(super) struct Orphaning {
    pub(super) fake: FakeRuntime,
    pub(super) pid: u32,
}

impl AgentRuntime for Orphaning {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let _ = events.send(RuntimeEvent::Launched {
            pid: Some(self.pid),
        });
        self.fake.execute(request, events, stop).await
    }
}

pub(super) fn agent_process() -> Child {
    let mut command = Command::new("git");
    command
        .args(["hash-object", "--stdin"])
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    command.spawn().expect("the agent process starts")
}

pub(super) fn exited(child: &mut Child) -> bool {
    let deadline = Instant::now() + EXIT_TIMEOUT;
    while Instant::now() < deadline {
        if child
            .try_wait()
            .expect("the child is inspectable")
            .is_some()
        {
            return true;
        }
        std::thread::sleep(RECORDING_POLL);
    }
    false
}

impl Feature {
    async fn recorded_pids(&self) -> Vec<(i64,)> {
        self.dir
            .rows("SELECT pid FROM execution WHERE pid IS NOT NULL")
            .await
    }

    async fn until_a_process_is_recorded(&self) {
        let deadline = Instant::now() + RECORDING_TIMEOUT;
        while self.recorded_pids().await.is_empty() {
            assert!(
                Instant::now() < deadline,
                "no process identity was recorded"
            );
            tokio::time::sleep(RECORDING_POLL).await;
        }
    }

    async fn attempts(&self) -> Vec<(i64, String)> {
        self.dir
            .rows("SELECT attempt, state FROM execution ORDER BY attempt")
            .await
    }

    pub(super) async fn dropped_mid_execution(&self) {
        let runtime = Started {
            fake: FakeRuntime::new(Script::new().on("write", 1, 1, Behavior::WaitUntilStopped)),
            launched: Notify::new(),
        };
        let (sink, _events) = mpsc::unbounded_channel();
        tokio::select! {
            outcome = self.project.drive(&self.run, &runtime, sink, StopSignal::new()) => {
                panic!("the waiting Execution never ends, got {outcome:?}");
            }
            () = runtime.launched.notified() => {}
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_driver_dropped_mid_execution_frees_the_run_and_its_execution_becomes_interrupted() {
    let feature = Feature::review().await;
    feature.dropped_mid_execution().await;

    let resumed = feature
        .drive(Script::new().on("write", 1, 2, draft("# Draft 1\n")))
        .await;

    assert!(matches!(
        resumed.outcome.expect("the run is free again"),
        DriveOutcome::AtGate { .. }
    ));
    assert_eq!(
        feature.attempts().await,
        [(1, "interrupted".to_owned()), (2, "succeeded".to_owned())]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_execution_whose_end_was_never_recorded_is_discarded_even_with_a_valid_delivery() {
    let feature = Feature::review().await;
    feature.dropped_mid_execution().await;
    let ids: Vec<(String,)> = feature.dir.rows("SELECT id FROM execution").await;
    let delivery = feature
        .dir
        .path()
        .join(".surt/runtime/executions")
        .join(&ids[0].0)
        .join("delivery");
    fs::create_dir_all(&delivery).expect("the delivery directory");
    fs::write(delivery.join("draft.md"), "# Unseen draft\n").expect("a valid delivery");

    let resumed = feature
        .drive(Script::new().on("write", 1, 2, draft("# Draft 1\n")))
        .await;

    assert!(matches!(
        resumed.outcome.expect("the run resumes"),
        DriveOutcome::AtGate { .. }
    ));
    assert_eq!(
        &resumed.progress[..2],
        [
            Progress::Recovering {
                driver: Some(std::process::id())
            },
            Progress::Discarded {
                step: label("write", Some(1)),
                attempt: 1
            },
        ]
    );
    let drafts: Vec<(Vec<u8>,)> = feature
        .dir
        .rows(
            "SELECT b.bytes FROM artifact_revision r JOIN artifact a ON a.id = r.artifact_id \
             JOIN revision_file f ON f.revision_id = r.id JOIN blob b ON b.sha256 = f.blob_sha256 \
             WHERE a.name = 'draft' ORDER BY r.number",
        )
        .await;
    assert_eq!(drafts, [(b"# Draft 1\n".to_vec(),)]);
}

impl Feature {
    async fn orphaned(&self) -> Child {
        let orphan = agent_process();
        let orphaning = Orphaning {
            fake: FakeRuntime::new(Script::new().on("write", 1, 1, Behavior::WaitUntilStopped)),
            pid: orphan.id(),
        };
        let stop = StopSignal::new();
        let (sink, _events) = mpsc::unbounded_channel();
        let forcing = async {
            self.until_a_process_is_recorded().await;
            stop.force();
        };
        let (killed, ()) = tokio::join!(
            self.project
                .drive(&self.run, &orphaning, sink, stop.clone()),
            forcing
        );
        assert_eq!(
            killed.expect("the forced driver returns"),
            DriveOutcome::Interrupted
        );
        orphan
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_stops_an_agent_left_running_by_a_dead_driver() {
    let feature = Feature::review().await;
    let mut orphan = feature.orphaned().await;

    let cancelled = feature.cancel().await;

    assert!(exited(&mut orphan), "the orphan was stopped");
    assert_eq!(
        cancelled.outcome.expect("cancel returns"),
        Cancellation::Cancelled
    );
    assert_eq!(
        cancelled.progress[0],
        Progress::OrphanStopped {
            step: label("write", Some(1)),
            attempt: 1,
            pid: orphan.id()
        }
    );
    assert_eq!(feature.attempts().await, [(1, "interrupted".to_owned())]);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_left_running_by_a_dead_driver_is_stopped_never_adopted_and_retried() {
    let feature = Feature::review().await;
    let mut orphan = feature.orphaned().await;
    assert!(orphan.try_wait().expect("inspectable").is_none());

    let resumed = feature
        .drive(Script::new().on("write", 1, 2, draft("# Draft 1\n")))
        .await;

    assert!(exited(&mut orphan), "the orphan was stopped");
    assert!(matches!(
        resumed.outcome.expect("the run resumes"),
        DriveOutcome::AtGate { .. }
    ));
    assert_eq!(
        &resumed.progress[..2],
        [
            Progress::Recovering {
                driver: Some(std::process::id())
            },
            Progress::OrphanStopped {
                step: label("write", Some(1)),
                attempt: 1,
                pid: orphan.id()
            },
        ]
    );
    assert_eq!(
        feature.attempts().await,
        [(1, "interrupted".to_owned()), (2, "succeeded".to_owned())]
    );
    let stopped: Vec<(String,)> = feature
        .dir
        .rows("SELECT kind FROM event WHERE kind = 'execution.orphan_stopped'")
        .await;
    assert_eq!(stopped.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn status_warns_about_an_agent_still_running_with_no_driver_and_leaves_it_alone() {
    let feature = Feature::review().await;
    let mut orphan = feature.orphaned().await;
    feature
        .dir
        .execute("UPDATE workflow_run SET driver_pid = NULL")
        .await;

    let statuses = feature
        .project
        .status(StatusScope::Active)
        .await
        .expect("status reads");

    assert!(orphan.try_wait().expect("inspectable").is_none());
    assert_eq!(statuses[0].states, [RunState::Interrupted]);
    assert_eq!(
        statuses[0].unattended,
        [Unattended {
            step: label("write", Some(1)),
            pid: orphan.id(),
        }]
    );
    assert_eq!(feature.attempts().await, [(1, "running".to_owned())]);
    orphan.kill().expect("the orphan is killed");
    assert!(exited(&mut orphan));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_driver_records_its_own_identity_and_a_clean_resume_reports_no_recovery() {
    let feature = Feature::review().await;
    feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await
        .outcome
        .expect("the draft reaches the gate");

    let again = feature.drive(Script::new()).await;

    assert!(again.progress.is_empty(), "{:?}", again.progress);
    let drivers: Vec<(Option<i64>, Option<i64>, Option<String>)> = feature
        .dir
        .rows(
            "SELECT driver_pid, driver_start_time, driver_boot_id FROM workflow_run \
             WHERE parent_step_run_id IS NULL",
        )
        .await;
    let (pid, start_time, boot_id) = &drivers[0];
    assert_eq!(*pid, Some(i64::from(std::process::id())));
    assert!(start_time.is_some());
    assert!(boot_id.as_deref().is_some_and(|id| !id.is_empty()));
}
