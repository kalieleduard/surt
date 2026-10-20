use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tokio::sync::mpsc;

use super::executions::{CONTEXT, DELTA, Execution, NOTE, PREVIOUS_EXECUTION, PREVIOUS_SESSION};
use super::process::{Run, parent_environment, run};
use super::streams::{Fixture, Stream, delivered_note};
use crate::project::GlobalConfig;
use crate::runtime::{
    AgentRuntime, ClaudeCodeRuntime, Conversation, Exit, RestartReason, RuntimeEvent, StopSignal,
    Verdict,
};

const PROBED_VERSION: &str = "2.1.304";
const NO_TRANSCRIPT: &str = "exit 1";
const NO_CONVERSATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/claude-code/resume-without-transcript.stderr.txt"
));

struct Probe {
    executable: PathBuf,
    record: PathBuf,
}

impl Probe {
    fn new(execution: &Execution, resumed: &Stream, resumed_tail: &str) -> Probe {
        let root = execution.root();
        let record = root.join("record");
        fs::create_dir(&record).expect("the record directory");
        let stderr = NO_CONVERSATION.trim_end();
        let script = format!(
            "#!/bin/sh\n\
             if [ \"$1\" = --version ]; then echo '{PROBED_VERSION} (Claude Code)'; exit 0; fi\n\
             launch=1\n\
             if [ -e '{record}/argv-1' ]; then launch=2; fi\n\
             printf '%s\\n' \"$@\" > '{record}/argv-'$launch\n\
             cat > '{record}/stdin-'$launch\n\
             printf '{note}' > '{delivery}/note.md'\n\
             case \" $* \" in\n\
             *' --resume '*)\n\
             cat '{resumed}'\n\
             if [ '{resumed_tail}' = '{NO_TRANSCRIPT}' ]; then echo '{stderr}' >&2; fi\n\
             {resumed_tail}\n\
             exit\n\
             ;;\n\
             esac\n\
             cat '{fresh}'\n",
            record = record.display(),
            note = NOTE.replace('\n', "\\n"),
            delivery = execution.request.delivery_dir.display(),
            resumed = written(&root.join("resumed.jsonl"), resumed).display(),
            fresh = written(&root.join("fresh.jsonl"), &delivered_note()).display(),
        );
        let executable = root.join("claude");
        fs::write(&executable, script).expect("the stub executable");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("an executable stub");
        Probe { executable, record }
    }

    fn runtime(&self) -> ClaudeCodeRuntime {
        ClaudeCodeRuntime::new(
            &self.executable,
            parent_environment(&[]),
            GlobalConfig::default().stop_grace_period,
        )
    }

    fn recorded(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.record.join(name)).ok()
    }

    fn argv(&self, launch: u32) -> Vec<String> {
        self.recorded(&format!("argv-{launch}"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

fn written(path: &Path, stream: &Stream) -> PathBuf {
    fs::write(path, stream.lines().join("\n") + "\n").expect("a replayed stream");
    path.to_owned()
}

fn continuing() -> Execution {
    Execution::new(delivered_note().session_id()).continuing()
}

fn launches(run: &Run) -> usize {
    run.events
        .iter()
        .filter(|event| matches!(event, RuntimeEvent::Launched { .. }))
        .count()
}

fn conversation(run: &Run) -> Option<Conversation> {
    run.events.iter().find_map(|event| match event {
        RuntimeEvent::Reported(report) => report.conversation.clone(),
        _ => None,
    })
}

fn has(arguments: &[String], flag: &str) -> bool {
    arguments.iter().any(|argument| argument == flag)
}

#[tokio::test]
async fn a_continuation_whose_transcript_is_gone_is_relaunched_fresh_and_delivers() {
    let execution = continuing();
    let resumed = Stream::captured(Fixture::ResumeWithoutTranscript);
    let probe = Probe::new(&execution, &resumed, NO_TRANSCRIPT);

    let run = run(&probe.runtime(), &execution, |_| {}).await;

    let completion = run.outcome.as_ref().expect("a completion");
    assert!(matches!(
        execution.verdict(completion),
        Verdict::Accepted(_)
    ));
    assert_eq!(launches(&run), 2);
    assert_eq!(
        conversation(&run),
        Some(Conversation::Restarted {
            reason: RestartReason::TranscriptMissing,
            previous: PREVIOUS_EXECUTION.to_owned(),
        })
    );
    let (continued, fresh) = (probe.argv(1), probe.argv(2));
    assert!(has(&continued, "--resume") && has(&continued, PREVIOUS_SESSION));
    assert!(has(&continued, "--fork-session"));
    assert!(!has(&fresh, "--resume") && !has(&fresh, "--fork-session"));
    let session = execution.request.session_id.to_string();
    assert!(has(&continued, &session) && has(&fresh, &session));
    assert_eq!(probe.recorded("stdin-1").as_deref(), Some(DELTA));
    assert_eq!(probe.recorded("stdin-2").as_deref(), Some(CONTEXT));
}

#[tokio::test]
async fn both_launches_of_a_restarted_execution_stay_in_its_logs() {
    let execution = continuing();
    let resumed = Stream::captured(Fixture::ResumeWithoutTranscript);
    let probe = Probe::new(&execution, &resumed, NO_TRANSCRIPT);

    run(&probe.runtime(), &execution, |_| {}).await;

    let logs = &execution.request.log_dir;
    let stream = fs::read_to_string(logs.join("stream.jsonl")).expect("the stream log");
    let launched = [resumed.lines(), delivered_note().lines()].concat();
    assert_eq!(stream, launched.join("\n") + "\n");
    let stderr = fs::read_to_string(logs.join("stderr.log")).expect("the stderr log");
    assert_eq!(stderr, NO_CONVERSATION);
}

#[tokio::test]
async fn a_continuation_that_initializes_is_launched_once_and_reported_continued() {
    let execution = continuing();
    let probe = Probe::new(&execution, &delivered_note(), "");

    let run = run(&probe.runtime(), &execution, |_| {}).await;

    let completion = run.outcome.as_ref().expect("a completion");
    assert!(matches!(
        execution.verdict(completion),
        Verdict::Accepted(_)
    ));
    assert_eq!(launches(&run), 1);
    assert_eq!(
        conversation(&run),
        Some(Conversation::ContinuedFrom(PREVIOUS_EXECUTION.to_owned()))
    );
    assert_eq!(probe.recorded("argv-2"), None);
}

#[tokio::test]
async fn a_stop_during_the_continued_launch_is_an_interruption_without_a_relaunch() {
    let execution = continuing();
    let resumed = Stream::captured(Fixture::ResumeWithoutTranscript);
    let probe = Probe::new(&execution, &resumed, "sleep 30");
    let stop = StopSignal::new();
    stop.graceful();
    let (sender, mut receiver) = mpsc::unbounded_channel();

    let outcome = probe
        .runtime()
        .execute(execution.request.clone(), sender, stop)
        .await;

    let events = std::iter::from_fn(|| receiver.try_recv().ok()).collect();
    let run = Run { outcome, events };
    assert_eq!(launches(&run), 1);
    assert_eq!(
        run.outcome.expect("an interrupted completion").exit,
        Exit::Stopped
    );
}
