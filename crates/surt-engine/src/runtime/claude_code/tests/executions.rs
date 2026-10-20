use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use uuid::Uuid;

use super::streams::Stream;
use crate::artifacts::DeliveryContract;
use crate::runtime::claude_code::{Invocation, Transcript};
use crate::runtime::{
    AgentLaunch, Exit, RuntimeCompletion, RuntimeError, RuntimeReport, RuntimeRequest, StepAttempt,
    Verdict, accept,
};
use crate::workflow::{AgentContract, AgentFiles};

pub(super) const NOTE: &str = "hello from fixture\n";
pub(super) const CONTEXT: &str = "Create note.md, then deliver.\n";
pub(super) const INSTRUCTIONS: &str = "Write output files only inside the delivery directory.\n";

pub(super) const NOTER_AGENT: &str = r#"
schema = 1
id = "noter"

[outputs.note]
format = "markdown"

[profile]
runtime = "claude-code"
model = "claude-haiku-5-5"
effort = "low"

[profile.claude]
auth = "subscription"
permission_mode = "dontAsk"
tools = ["Read", "Write"]
allowed_tools = ["Read"]
disallowed_tools = ["Bash"]
"#;

pub(super) struct Execution {
    dir: TempDir,
    pub(super) request: RuntimeRequest,
    contract: DeliveryContract,
}

impl Execution {
    pub(super) fn new(session_id: Uuid) -> Execution {
        Execution::with_agent(NOTER_AGENT, session_id)
    }

    pub(super) fn with_agent(toml: &'static str, session_id: Uuid) -> Execution {
        let dir = TempDir::new().expect("a temporary directory");
        let root = dunce::canonicalize(dir.path()).expect("a canonical temporary directory");
        let delivery_dir = root.join("delivery");
        let working_dir = root.join("worktree");
        fs::create_dir(&delivery_dir).expect("the delivery directory");
        fs::create_dir(&working_dir).expect("the working directory");
        let assets = AgentFiles::of(toml, INSTRUCTIONS, &[]);
        let agent =
            AgentContract::parse("noter", &assets.toml, &assets).expect("a valid agent contract");
        let contract =
            DeliveryContract::new(&agent, |path| assets.schema(path)).expect("compilable schemas");
        let launch = AgentLaunch::new(&agent, INSTRUCTIONS, working_dir, root.join("launch"));
        let request = RuntimeRequest {
            step: StepAttempt::new("note", 1, 1),
            session_id,
            model: agent.profile.model.clone(),
            delivery_dir,
            log_dir: root.join("logs"),
            context: CONTEXT.to_owned(),
            continuation: None,
            launch,
        };
        Execution {
            dir,
            request,
            contract,
        }
    }

    pub(super) fn root(&self) -> PathBuf {
        dunce::canonicalize(self.dir.path()).expect("a canonical temporary directory")
    }

    pub(super) fn deliver_note(&self) {
        fs::write(self.request.delivery_dir.join("note.md"), NOTE).expect("the note");
    }

    pub(super) fn invocation(&self, environment: &[(&str, &str)]) -> Invocation {
        let environment: Vec<(OsString, OsString)> = environment
            .iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value)))
            .collect();
        Invocation::assemble(&self.request, &environment).expect("an invocation")
    }

    pub(super) fn with_model(mut self, model: &str) -> Execution {
        self.request.model = model.to_owned();
        self
    }

    pub(super) fn replay(
        &self,
        stream: &Stream,
        exit: Exit,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        self.replay_lines(&stream.lines(), exit)
    }

    pub(super) fn replay_lines(
        &self,
        lines: &[String],
        exit: Exit,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let mut transcript = Transcript::new(self.invocation(&[]).expectation);
        for line in lines {
            transcript.observe(line.as_bytes());
        }
        transcript.conclude(exit)
    }

    pub(super) fn report(&self, stream: &Stream, exit: Exit) -> RuntimeReport {
        let mut transcript = Transcript::new(self.invocation(&[]).expectation);
        for line in stream.lines() {
            transcript.observe(line.as_bytes());
        }
        transcript.report(exit)
    }

    pub(super) fn failure(&self, stream: &Stream, exit: Exit) -> RuntimeError {
        self.replay(stream, exit).expect_err("a classified failure")
    }

    pub(super) fn verdict(&self, completion: &RuntimeCompletion) -> Verdict {
        accept(&self.request, completion, &self.contract)
    }
}
