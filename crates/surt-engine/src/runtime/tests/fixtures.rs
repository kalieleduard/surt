use std::path::{Path, PathBuf};

use tempfile::TempDir;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::artifacts::{DeliveryContract, DeliveryRejection};
use crate::runtime::{
    AgentLaunch, AgentRuntime, Behavior, Delivery, FailureClass, FakeRuntime, Rejection,
    RuntimeCompletion, RuntimeError, RuntimeEvent, RuntimeReport, RuntimeRequest, Script,
    StepAttempt, StopSignal, Verdict, accept,
};
use crate::workflow::{AgentAssets, AgentContract};

pub(super) const STEP: &str = "write";
pub(super) const MODEL: &str = "claude-sonnet-5-5";
pub(super) const REPORT: &str = "# Report\n\nAll good.\n";
pub(super) const DATA: &str = r#"{ "count": 2 }"#;

const WRITER_AGENT: &str = r#"
schema = 1
id = "writer"
outcomes = ["approved", "changes_requested"]

[outputs.report]
format = "markdown"

[outputs.data]
format = "json"
json_schema = "schemas/data.schema.json"

[profile]
runtime = "fake"
model = "claude-sonnet-5-5"
effort = "high"
"#;

const DATA_SCHEMA: &str = r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "additionalProperties": false,
  "required": ["count"],
  "properties": { "count": { "type": "integer", "minimum": 1 } }
}"#;

pub(super) struct Execution {
    _dir: TempDir,
    pub(super) request: RuntimeRequest,
    contract: DeliveryContract,
}

impl Execution {
    pub(super) fn new() -> Execution {
        Execution::with_contract(writer_contract())
    }

    pub(super) fn with_contract(contract: DeliveryContract) -> Execution {
        let dir = TempDir::new().expect("a temporary directory");
        let delivery_dir = dir.path().join("delivery");
        std::fs::create_dir(&delivery_dir).expect("the delivery directory");
        let request = RuntimeRequest {
            step: StepAttempt::new(STEP, 1, 1),
            session_id: Uuid::now_v7(),
            model: MODEL.to_owned(),
            log_dir: dir.path().join("logs"),
            context: String::new(),
            launch: writer_launch(dir.path()),
            delivery_dir,
        };
        Execution {
            _dir: dir,
            request,
            contract,
        }
    }

    pub(super) fn delivery_dir(&self) -> PathBuf {
        self.request.delivery_dir.clone()
    }

    pub(super) async fn run(&self, behavior: Behavior) -> Result<Verdict, RuntimeError> {
        let completion = self.complete(behavior).await?;
        Ok(self.verdict(&completion))
    }

    pub(super) async fn complete(
        &self,
        behavior: Behavior,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        self.complete_script(Script::new().on(STEP, 1, 1, behavior), StopSignal::new())
            .await
    }

    pub(super) async fn complete_script(
        &self,
        script: Script,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let (events, _received) = mpsc::unbounded_channel();
        FakeRuntime::new(script)
            .execute(self.request.clone(), events, stop)
            .await
    }

    pub(super) async fn report(&self, behavior: Behavior) -> Option<RuntimeReport> {
        let (events, mut received) = mpsc::unbounded_channel();
        let script = Script::new().on(STEP, 1, 1, behavior);
        let _ = FakeRuntime::new(script)
            .execute(self.request.clone(), events, StopSignal::new())
            .await;
        let mut report = None;
        while let Ok(event) = received.try_recv() {
            if let RuntimeEvent::Reported(reported) = event {
                report = Some(reported);
            }
        }
        report
    }

    pub(super) fn verdict(&self, completion: &RuntimeCompletion) -> Verdict {
        accept(&self.request, completion, &self.contract)
    }

    pub(super) fn rejection(&self, completion: &RuntimeCompletion) -> DeliveryRejection {
        let verdict = self.verdict(completion);
        let Verdict::Rejected(rejection) = verdict else {
            panic!("expected a rejected delivery, got {verdict:?}");
        };
        assert_eq!(rejection.class(), FailureClass::Retryable);
        let Rejection::Delivery(rejection) = rejection else {
            panic!("expected a rejected delivery, got {rejection:?}");
        };
        rejection
    }

    pub(super) async fn delivery_rejection(&self, delivery: Delivery) -> DeliveryRejection {
        let completion = self
            .complete(Behavior::Deliver(delivery))
            .await
            .expect("the fake completes");
        self.rejection(&completion)
    }
}

pub(super) fn valid(outcome: &str) -> Delivery {
    Delivery::new(outcome)
        .output("report", "report.md", REPORT)
        .output("data", "out/data.json", DATA)
}

pub(super) async fn rejection(delivery: Delivery) -> DeliveryRejection {
    Execution::new().delivery_rejection(delivery).await
}

fn writer_launch(dir: &Path) -> AgentLaunch {
    let assets = AgentAssets {
        toml: WRITER_AGENT,
        instructions: "Write.",
        schemas: &[("schemas/data.schema.json", DATA_SCHEMA)],
    };
    let agent =
        AgentContract::parse("writer", assets.toml, &assets).expect("a valid agent contract");
    AgentLaunch::new(
        &agent,
        assets.instructions,
        dir.to_owned(),
        dir.join("launch"),
    )
}

pub(super) fn writer_contract() -> DeliveryContract {
    contract(
        "writer",
        WRITER_AGENT,
        &[("schemas/data.schema.json", DATA_SCHEMA)],
    )
}

pub(super) fn contract(
    id: &str,
    toml: &'static str,
    schemas: &'static [(&'static str, &'static str)],
) -> DeliveryContract {
    let assets = AgentAssets {
        toml,
        instructions: "Write.",
        schemas,
    };
    let agent = AgentContract::parse(id, assets.toml, &assets).expect("a valid agent contract");
    DeliveryContract::new(&agent, |path| assets.schema(path)).expect("compilable schemas")
}
