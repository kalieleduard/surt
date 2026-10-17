use std::time::{SystemTime, UNIX_EPOCH};

use super::{Error, Project, Refusal};
use crate::store::{NewInitialArtifact, NewRunRows, RunCreation};
use crate::workflow::freeze;

#[derive(Clone, Debug)]
pub struct NewRun {
    pub subject: String,
    pub workflow: String,
    pub initial_artifacts: Vec<InitialArtifact>,
    pub actor: String,
}

#[derive(Clone, Debug)]
pub struct InitialArtifact {
    pub name: String,
    pub files: Vec<ArtifactFile>,
}

#[derive(Clone, Debug)]
pub struct ArtifactFile {
    pub rel_path: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RunId(String);

impl RunId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Project {
    pub async fn create_run(&self, new: NewRun) -> Result<RunId, Error> {
        let names = new
            .initial_artifacts
            .iter()
            .map(|artifact| artifact.name.as_str());
        let frozen = freeze(&self.catalog, &new.workflow, names, &self.config)?;
        let rows = NewRunRows {
            subject: new.subject,
            frozen,
            initial_artifacts: new
                .initial_artifacts
                .into_iter()
                .map(|artifact| NewInitialArtifact {
                    name: artifact.name,
                    files: artifact
                        .files
                        .into_iter()
                        .map(|file| (file.rel_path, file.bytes))
                        .collect(),
                })
                .collect(),
            actor: new.actor,
            now: unix_now(),
        };
        match self.store.create_run(rows).await? {
            RunCreation::Created(run) => Ok(RunId(run)),
            RunCreation::ActiveRunExists(run) => {
                Err(Error::Refused(Refusal::ActiveRunExists { run: RunId(run) }))
            }
        }
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}
