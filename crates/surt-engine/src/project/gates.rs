use super::{Error, Project, Refusal, RunId, open_store};
use crate::drive::{CorruptSnapshot, Frozen};
use crate::store::{Decision, DecisionOutcome, NewDecision, Store};
use crate::workflow::StepSpec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decided {
    pub gate: String,
    pub artifact: String,
    pub revision: u32,
    pub sha256: String,
    pub origin: RevisionOrigin,
    pub actor: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevisionOrigin {
    Agent,
    Human,
}

struct Request<'a> {
    root: &'a str,
    expected: Option<u32>,
    actor: &'a str,
    reason: Option<&'a str>,
}

impl Project {
    pub async fn approve(
        &self,
        run: &RunId,
        revision: Option<u32>,
        actor: &str,
    ) -> Result<Decided, Error> {
        self.decide_gate(Request {
            root: run.as_str(),
            expected: revision,
            actor,
            reason: None,
        })
        .await
    }

    pub async fn reject(
        &self,
        run: &RunId,
        revision: Option<u32>,
        reason: &str,
        actor: &str,
    ) -> Result<Decided, Error> {
        self.decide_gate(Request {
            root: run.as_str(),
            expected: revision,
            actor,
            reason: Some(reason),
        })
        .await
    }

    async fn decide_gate(&self, request: Request<'_>) -> Result<Decided, Error> {
        let store = open_store(&self.layout).await?;
        let decided = record(&store, &request).await;
        store.close().await;
        decided
    }
}

async fn record(store: &Store, request: &Request<'_>) -> Result<Decided, Error> {
    let gates = store.waiting_gates(request.root).await?;
    let Some(gate) = gates.first() else {
        return Err(no_gate(store, request.root).await?);
    };
    let frozen = Frozen::from_files(store.frozen_files(request.root).await?)?;
    let definition = frozen.definition(&gate.definition)?;
    let Some(StepSpec::Approval(spec)) = definition.step(&gate.key).map(|step| &step.spec) else {
        return Err(CorruptSnapshot::new(&gate.key, "not an approval step").into());
    };
    let decision = match request.reason {
        None => Decision::Approve,
        Some(reason) => Decision::Reject {
            rejection_artifact: &spec.rejection_artifact,
            reason,
        },
    };
    let outcome = store
        .record_decision(NewDecision {
            gate,
            artifact: &spec.artifact,
            expected: request.expected,
            decision,
            actor: request.actor,
        })
        .await?;
    match outcome {
        DecisionOutcome::Recorded {
            number,
            identity,
            from_execution,
        } => Ok(Decided {
            gate: gate.key.clone(),
            artifact: spec.artifact.clone(),
            revision: number,
            sha256: identity,
            origin: if from_execution {
                RevisionOrigin::Agent
            } else {
                RevisionOrigin::Human
            },
            actor: request.actor.to_owned(),
        }),
        DecisionOutcome::Stale { expected, latest } => Err(Refusal::StaleRevision {
            subject: store.subject_of_run(request.root).await?,
            artifact: spec.artifact.clone(),
            expected,
            latest,
        }
        .into()),
        DecisionOutcome::GateNotWaiting | DecisionOutcome::NoRevision => {
            Err(no_gate(store, request.root).await?)
        }
    }
}

async fn no_gate(store: &Store, root: &str) -> Result<Error, Error> {
    Ok(Refusal::NoGateWaiting {
        subject: store.subject_of_run(root).await?,
    }
    .into())
}
