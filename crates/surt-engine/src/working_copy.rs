mod candidate;
mod disk;
mod pending;
mod revalidation;

use std::io;
use std::path::{Path, PathBuf};

use crate::artifacts::{DeliveryRejection, OutputRule};
use crate::drive::{CorruptSnapshot, Frozen};
use crate::project::{Error, Layout, Refusal};
use crate::store::{BaselineFile, EditRecord, Store, WorkingCopyArtifact};
use crate::workflow::{Definition, StepSpec};
use candidate::Candidate;
use disk::Inspection;

pub(crate) use pending::finish_pending_writes;

pub(crate) fn unrecorded_edits(dir: &Path, files: &[BaselineFile]) -> io::Result<Vec<String>> {
    Ok(disk::inspect(dir, files)?.edited())
}

pub(crate) struct WorkingCopy {
    dir: PathBuf,
    root: String,
    subject: String,
    gated: Vec<Gated>,
}

struct Gated {
    artifact: String,
    gate: String,
    rule: OutputRule,
}

#[derive(Debug, Default)]
pub(crate) struct Scanned {
    pub(crate) recorded: Vec<RecordedEdit>,
    pub(crate) restored: Vec<RestoredFile>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RestoredFile {
    pub(crate) artifact: String,
    pub(crate) path: String,
    pub(crate) revision: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RecordedEdit {
    pub(crate) artifact: String,
    pub(crate) revision: u32,
}

impl WorkingCopy {
    pub(crate) async fn load(
        store: &Store,
        layout: &Layout,
        root: &str,
        frozen: &Frozen,
    ) -> Result<WorkingCopy, Error> {
        let subject = store.subject_of_run(root).await?;
        let definition = frozen.definition(&store.run_definition(root).await?)?;
        Ok(WorkingCopy {
            dir: layout.working_copy(&subject),
            root: root.to_owned(),
            subject,
            gated: gated(frozen, definition)?,
        })
    }

    pub(crate) async fn record(&self, store: &Store, actor: &str) -> Result<Scanned, Error> {
        self.scan(store, actor, |_| true).await
    }

    pub(crate) async fn record_artifacts(
        &self,
        store: &Store,
        actor: &str,
        artifacts: &[&str],
    ) -> Result<Scanned, Error> {
        self.scan(store, actor, |name| artifacts.contains(&name))
            .await
    }

    async fn scan(
        &self,
        store: &Store,
        actor: &str,
        wanted: impl Fn(&str) -> bool,
    ) -> Result<Scanned, Error> {
        let mut scanned = Scanned::default();
        for artifact in store.working_copy(&self.root).await? {
            let Some(gated) = self.gated(&artifact).filter(|_| wanted(&artifact.name)) else {
                continue;
            };
            let inspection = disk::inspect(&self.dir, &artifact.files)
                .map_err(|error| Error::io("read", &self.dir, error))?;
            if inspection.is_edit() {
                let recorded = self
                    .record_edit(store, gated, &artifact, &inspection, actor)
                    .await?;
                scanned.recorded.extend(recorded);
            } else {
                self.restore(store, &artifact, &inspection, &mut scanned)
                    .await?;
            }
        }
        Ok(scanned)
    }

    async fn record_edit(
        &self,
        store: &Store,
        gated: &Gated,
        artifact: &WorkingCopyArtifact,
        inspection: &Inspection,
        actor: &str,
    ) -> Result<Option<RecordedEdit>, Error> {
        if let Some(revision) = artifact.pinned {
            return Err(self.pinned(artifact, revision, inspection));
        }
        let files = self.candidate(store, gated, artifact, inspection).await?;
        let record = EditRecord {
            artifact,
            files: &files,
            actor,
        };
        Ok(store
            .record_edit(record)
            .await?
            .map(|revision| RecordedEdit {
                artifact: artifact.name.clone(),
                revision,
            }))
    }

    async fn restore(
        &self,
        store: &Store,
        artifact: &WorkingCopyArtifact,
        inspection: &Inspection,
        scanned: &mut Scanned,
    ) -> Result<(), Error> {
        for file in &artifact.files {
            if !inspection.missing.contains(&file.rel_path) {
                continue;
            }
            let bytes = store.blob(&file.sha256).await?;
            let target = self.dir.join(&file.rel_path);
            disk::write(&target, &bytes).map_err(|error| Error::io("write", &target, error))?;
            scanned.restored.push(RestoredFile {
                artifact: artifact.name.clone(),
                path: file.rel_path.clone(),
                revision: artifact.revision,
            });
        }
        Ok(())
    }

    fn gated(&self, artifact: &WorkingCopyArtifact) -> Option<&Gated> {
        if artifact.pending {
            return None;
        }
        self.gated
            .iter()
            .find(|gated| gated.artifact == artifact.name)
    }

    async fn candidate(
        &self,
        store: &Store,
        gated: &Gated,
        artifact: &WorkingCopyArtifact,
        inspection: &Inspection,
    ) -> Result<Vec<(String, Vec<u8>)>, Error> {
        let index = disk::index(store, &artifact.files).await?;
        let candidate = Candidate {
            dir: &self.dir,
            index: &index,
            baseline: &artifact.files,
            inspection,
        };
        candidate
            .validate(&gated.rule)
            .map_err(|rejection| self.invalid(artifact, inspection, &rejection))
    }

    fn pinned(
        &self,
        artifact: &WorkingCopyArtifact,
        revision: u32,
        inspection: &Inspection,
    ) -> Error {
        Refusal::PinnedEdit {
            subject: self.subject.clone(),
            artifact: artifact.name.clone(),
            revision,
            files: inspection.edited(),
        }
        .into()
    }

    fn invalid(
        &self,
        artifact: &WorkingCopyArtifact,
        inspection: &Inspection,
        rejection: &DeliveryRejection,
    ) -> Error {
        Refusal::InvalidEdit {
            subject: self.subject.clone(),
            artifact: artifact.name.clone(),
            problem: rejection.to_string(),
            files: inspection.edited(),
        }
        .into()
    }
}

fn gated(frozen: &Frozen, definition: &Definition) -> Result<Vec<Gated>, CorruptSnapshot> {
    let mut gated = Vec::new();
    for step in &definition.steps {
        let StepSpec::Approval(gate) = &step.spec else {
            continue;
        };
        if let Some(rule) = producer_rule(frozen, definition, &gate.artifact)? {
            gated.push(Gated {
                artifact: gate.artifact.clone(),
                gate: step.key.clone(),
                rule,
            });
        }
    }
    Ok(gated)
}

fn producer_rule(
    frozen: &Frozen,
    definition: &Definition,
    artifact: &str,
) -> Result<Option<OutputRule>, CorruptSnapshot> {
    for step in &definition.steps {
        let StepSpec::Agent(spec) = &step.spec else {
            continue;
        };
        let agent = frozen.agent(&spec.agent)?;
        if agent.contract.outputs.contains_key(artifact) {
            return Ok(agent.delivery_contract()?.into_output(artifact));
        }
    }
    Ok(None)
}
