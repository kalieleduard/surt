use super::{Error, Project, Refusal, RunId, open_store};
use crate::drive::{CorruptSnapshot, Frozen};
use crate::store::{Decision, DecisionOutcome, NewDecision, Store, WaitingGate};
use crate::workflow::StepSpec;
use crate::working_copy::WorkingCopy;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decided {
    pub gate: String,
    pub artifact: String,
    pub revision: u32,
    pub sha256: String,
    pub origin: RevisionOrigin,
    pub actor: String,
    pub files: Vec<String>,
    pub restored: Vec<String>,
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
        let decided = self.scan_and_record(&store, &request).await;
        store.close().await;
        decided
    }

    async fn scan_and_record(
        &self,
        store: &Store,
        request: &Request<'_>,
    ) -> Result<Decided, Error> {
        let frozen = Frozen::from_files(store.frozen_files(request.root).await?)?;
        let working_copy = WorkingCopy::load(store, &self.layout, request.root, &frozen).await?;
        let scanned = working_copy.record(store, request.actor).await?;
        let gates = store.waiting_gates(request.root).await?;
        let decision = match gates.first() {
            Some(gate) => decide_waiting(store, request, gate).await?,
            None => approve_after_gate(store, request, &working_copy).await?,
        };
        let mut decided = decided(store, request, decision).await?;
        decided.restored = scanned.restored.into_iter().map(|file| file.path).collect();
        Ok(decided)
    }
}

struct Recorded {
    gate: String,
    artifact: String,
    outcome: DecisionOutcome,
}

async fn decide_waiting(
    store: &Store,
    request: &Request<'_>,
    gate: &WaitingGate,
) -> Result<Recorded, Error> {
    let spec = approval_step(store, request.root, &gate.definition, &gate.key).await?;
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
    Ok(Recorded {
        gate: gate.key.clone(),
        artifact: spec.artifact,
        outcome,
    })
}

async fn approve_after_gate(
    store: &Store,
    request: &Request<'_>,
    working_copy: &WorkingCopy,
) -> Result<Recorded, Error> {
    let unapproved = match request.reason {
        None => working_copy.unapproved(store).await?,
        Some(_) => None,
    };
    let Some(unapproved) = unapproved else {
        return Err(no_gate(store, request.root).await?);
    };
    let outcome = store
        .approve_latest(
            request.root,
            &unapproved.artifact,
            request.expected,
            request.actor,
        )
        .await?;
    Ok(Recorded {
        gate: unapproved.gate,
        artifact: unapproved.artifact,
        outcome,
    })
}

async fn decided(
    store: &Store,
    request: &Request<'_>,
    recorded: Recorded,
) -> Result<Decided, Error> {
    match recorded.outcome {
        DecisionOutcome::Recorded {
            revision_id,
            number,
            identity,
            from_execution,
        } => Ok(Decided {
            gate: recorded.gate,
            artifact: recorded.artifact,
            revision: number,
            sha256: identity,
            origin: if from_execution {
                RevisionOrigin::Agent
            } else {
                RevisionOrigin::Human
            },
            actor: request.actor.to_owned(),
            files: store.revision_paths(&revision_id).await?,
            restored: Vec::new(),
        }),
        DecisionOutcome::Stale { expected, latest } => Err(Refusal::StaleRevision {
            subject: store.subject_of_run(request.root).await?,
            artifact: recorded.artifact,
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

pub(super) struct GateArtifacts {
    pub(super) artifact: String,
    rejection_artifact: String,
}

pub(super) async fn approval_step(
    store: &Store,
    root: &str,
    definition: &str,
    key: &str,
) -> Result<GateArtifacts, Error> {
    let frozen = Frozen::from_files(store.frozen_files(root).await?)?;
    let definition = frozen.definition(definition)?;
    let Some(StepSpec::Approval(spec)) = definition.step(key).map(|step| &step.spec) else {
        return Err(CorruptSnapshot::new(key, "not an approval step").into());
    };
    Ok(GateArtifacts {
        artifact: spec.artifact.clone(),
        rejection_artifact: spec.rejection_artifact.clone(),
    })
}
