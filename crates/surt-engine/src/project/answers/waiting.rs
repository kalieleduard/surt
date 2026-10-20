use crate::drive::{CorruptSnapshot, Frozen};
use crate::project::{Error, Project, Questions, Refusal, RunId};
use crate::store::{Store, WaitingGate};
use crate::workflow::StepSpec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaitingInput {
    pub step: String,
    pub artifact: String,
    pub revision: u32,
    pub round: u32,
    pub questions: Questions,
}

pub(in crate::project) struct InputArtifacts {
    pub(in crate::project) artifact: String,
    pub(in crate::project) answer_artifact: String,
}

pub(super) struct Waiting {
    pub(super) gate: WaitingGate,
    pub(super) spec: InputArtifacts,
    pub(super) input: WaitingInput,
}

impl Project {
    pub async fn waiting_input(&self, run: &RunId) -> Result<WaitingInput, Error> {
        let store = self.reader().await?;
        let waiting = self.waiting(&store, run.as_str()).await;
        store.close().await;
        Ok(waiting?.input)
    }

    pub(super) async fn waiting(&self, store: &Store, root: &str) -> Result<Waiting, Error> {
        let gates = store.waiting_gates(root).await?;
        let Some(gate) = gates.into_iter().find(WaitingGate::is_input) else {
            return Err(self.no_input(store, root).await?);
        };
        let spec = input_step(store, root, &gate.definition, &gate.key).await?;
        let Some(id) = store.revision_id(&gate.run, &spec.artifact, None).await? else {
            return Err(self.no_input(store, root).await?);
        };
        let revision = store.revision(&id).await?;
        let questions = revision
            .files
            .iter()
            .find_map(|(_, bytes)| Questions::parse(bytes))
            .ok_or_else(|| CorruptSnapshot::new(&spec.artifact, "the questions do not parse"))?;
        let input = WaitingInput {
            step: gate.key.clone(),
            artifact: spec.artifact.clone(),
            revision: revision.number,
            round: next_round(store, &gate.run, &spec.answer_artifact).await?,
            questions,
        };
        Ok(Waiting { gate, spec, input })
    }

    pub(super) async fn no_input(&self, store: &Store, root: &str) -> Result<Error, Error> {
        let subject = store.subject_of_run(root).await?;
        let mut states = Vec::new();
        for run in store.root_runs(&subject).await? {
            if run.id == root {
                states = self.run_status(store, &run).await?.states;
            }
        }
        Ok(Refusal::NoInputWaiting { subject, states }.into())
    }
}

pub(in crate::project) async fn input_step(
    store: &Store,
    root: &str,
    definition: &str,
    key: &str,
) -> Result<InputArtifacts, Error> {
    let frozen = Frozen::from_files(store.frozen_files(root).await?)?;
    let definition = frozen.definition(definition)?;
    let Some(StepSpec::Input(spec)) = definition.step(key).map(|step| &step.spec) else {
        return Err(CorruptSnapshot::new(key, "not an input step").into());
    };
    Ok(InputArtifacts {
        artifact: spec.artifact.clone(),
        answer_artifact: spec.answer_artifact.clone(),
    })
}

pub(in crate::project) async fn next_round(
    store: &Store,
    run: &str,
    answer_artifact: &str,
) -> Result<u32, Error> {
    let answered = store.latest_revision_number(run, answer_artifact).await?;
    Ok(answered.unwrap_or_default() + 1)
}
