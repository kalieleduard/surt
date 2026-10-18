use std::sync::Mutex;

use tokio::sync::mpsc;

use super::super::fixtures::{SUBJECT, default_config};
use super::fixtures::{ACTOR, Feature, PUBLISHER, WRITER, agent, draft, label, summary};
use crate::project::{Catalog, DriveOutcome, Error, Gate, Project, Refusal, RunId};
use crate::runtime::{
    AgentRuntime, FakeRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink, RuntimeRequest,
    Script, StopSignal,
};

const PARALLEL_WORKFLOW: &str = r#"
schema = 1
name = "parallel"

[steps.write]
kind = "agent"
agent = "writer"

[steps.approve]
kind = "approval"
artifact = "draft"
rejection_artifact = "draft-rejection"
needs = ["write"]

[steps.publish]
kind = "agent"
agent = "publisher"
needs = ["approve"]
when = { step = "approve", outcome = "approved" }

[steps.side]
kind = "agent"
agent = "publisher"
needs = ["write"]
"#;

struct Meddler {
    other: Project,
    run: RunId,
    fake: FakeRuntime,
    meddled: Mutex<Vec<Result<String, Error>>>,
}

enum Meddling {
    Drive,
    Approve,
}

impl Meddler {
    fn outcomes(&self) -> Vec<Result<String, Error>> {
        std::mem::take(&mut *self.meddled.lock().expect("the outcomes"))
    }

    async fn meddle(&self, meddling: Meddling) -> Result<String, Error> {
        match meddling {
            Meddling::Drive => {
                let (sink, _events) = mpsc::unbounded_channel();
                let outcome = self
                    .other
                    .drive(&self.run, &self.fake, sink, StopSignal::new())
                    .await?;
                Ok(format!("{outcome:?}"))
            }
            Meddling::Approve => {
                let decided = self.other.approve(&self.run, None, ACTOR).await?;
                Ok(decided.gate)
            }
        }
    }
}

struct Meddle {
    meddler: Meddler,
    on_step: &'static str,
    meddling: fn() -> Meddling,
}

impl AgentRuntime for Meddle {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        if request.step.step == self.on_step {
            let outcome = self.meddler.meddle((self.meddling)()).await;
            self.meddler
                .meddled
                .lock()
                .expect("the outcomes")
                .push(outcome);
        }
        self.meddler.fake.execute(request, events, stop).await
    }
}

async fn meddle(
    feature: &Feature,
    catalog: Catalog,
    script: Script,
    on_step: &'static str,
    meddling: fn() -> Meddling,
) -> Meddle {
    Meddle {
        meddler: Meddler {
            other: feature.dir.project(catalog, default_config()).await,
            run: feature.run.clone(),
            fake: FakeRuntime::new(script),
            meddled: Mutex::new(Vec::new()),
        },
        on_step,
        meddling,
    }
}

fn parallel_catalog() -> Catalog {
    Catalog::new()
        .workflow("parallel", PARALLEL_WORKFLOW)
        .agent("writer", agent(WRITER))
        .agent("publisher", agent(PUBLISHER))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_driver_refuses_with_the_live_drivers_pid_and_the_lock_is_released_on_return() {
    let feature = Feature::review().await;
    let script = Script::new().on("write", 1, 1, draft("# Draft 1\n"));
    let runtime = meddle(
        &feature,
        super::fixtures::review_catalog(),
        script,
        "write",
        || Meddling::Drive,
    )
    .await;

    let first = feature.drive_with(&runtime).await;
    let again = feature.drive(Script::new()).await;

    let meddled = runtime.meddler.outcomes();
    let [
        Err(Error::Refused(Refusal::DriverLive {
            subject,
            pid,
            since,
        })),
    ] = meddled.as_slice()
    else {
        panic!("expected a live-driver refusal, got {meddled:?}");
    };
    assert_eq!(subject, SUBJECT);
    assert_eq!(*pid, Some(std::process::id()));
    assert!(since.is_some());
    let at_gate = DriveOutcome::AtGate {
        gates: vec![Gate {
            step: label("approve", Some(1)),
            artifact: "draft".to_owned(),
        }],
    };
    assert_eq!(first.outcome.expect("the first driver returns"), at_gate);
    assert_eq!(
        again.outcome.expect("a later driver takes the lock"),
        at_gate
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_approval_recorded_elsewhere_while_driving_is_seen_on_the_next_tick() {
    let feature = Feature::new(parallel_catalog(), "parallel").await;
    let script = Script::new()
        .on("write", 1, 1, draft("# Draft 1\n"))
        .on("side", 1, 1, summary("side\n"))
        .on("publish", 1, 1, summary("published\n"));
    let runtime = meddle(&feature, parallel_catalog(), script, "side", || {
        Meddling::Approve
    })
    .await;

    let driven = feature.drive_with(&runtime).await;

    let meddled = runtime.meddler.outcomes();
    assert!(
        matches!(meddled.as_slice(), [Ok(gate)] if gate == "approve"),
        "{meddled:?}"
    );
    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let steps: Vec<(String, String)> = feature
        .dir
        .rows("SELECT step_key, state FROM step_run ORDER BY step_key")
        .await;
    assert_eq!(
        steps,
        [
            ("approve".to_owned(), "done".to_owned()),
            ("publish".to_owned(), "done".to_owned()),
            ("side".to_owned(), "done".to_owned()),
            ("write".to_owned(), "done".to_owned()),
        ]
    );
}
