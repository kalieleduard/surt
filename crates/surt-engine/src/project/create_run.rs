mod previous;

use std::num::NonZeroU32;

use serde::Serialize;

use super::{Check, Config, Error, GitConfig, Project, Refusal};
use crate::cleanup::clear_kept_working_copy;
pub use crate::drive::graph::RunId;
use crate::store::{NewInitialArtifact, NewRunRows, RunCreation, Store, unix_now};
use crate::workflow::{CleanupPolicy, FrozenRun, OnSuccess, freeze};

#[derive(Clone, Debug)]
pub struct NewRun {
    pub subject: String,
    pub workflow: String,
    pub initial_artifacts: Vec<InitialArtifact>,
    pub carried: Vec<String>,
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

#[derive(Serialize)]
struct FrozenConfig<'a> {
    max_attempts: NonZeroU32,
    max_iterations: NonZeroU32,
    git: &'a GitConfig,
    cleanup: CleanupPolicy,
    checks: &'a [Check],
}

impl Project {
    pub async fn create_run(&self, new: NewRun) -> Result<RunId, Error> {
        self.refuse_agent_overrides()?;
        let store = self.store().await?;
        let created = self.create_with(&store, new).await;
        store.close().await;
        created
    }

    async fn create_with(&self, store: &Store, mut new: NewRun) -> Result<RunId, Error> {
        let carried = previous::carried(store, &self.layout, &new.subject, &new.carried).await?;
        new.initial_artifacts.extend(carried.artifacts);
        let names = new
            .initial_artifacts
            .iter()
            .map(|artifact| artifact.name.as_str());
        let frozen = freeze(&self.catalog, &new.workflow, names, |root| {
            frozen_config(&self.config, root)
        })?;
        if let Some(previous) = &carried.previous {
            clear_kept_working_copy(store, &self.layout, previous).await?;
        }
        let subject = new.subject.clone();
        match store.create_run(rows(new, frozen)).await? {
            RunCreation::Created(run) => Ok(RunId(run)),
            RunCreation::ActiveRunExists(_) => {
                Err(Error::Refused(Refusal::ActiveRunExists { subject }))
            }
        }
    }
}

fn rows(new: NewRun, frozen: FrozenRun) -> NewRunRows {
    NewRunRows {
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
    }
}

fn frozen_config(config: &Config, root: OnSuccess) -> Result<String, toml::ser::Error> {
    toml::to_string(&FrozenConfig {
        max_attempts: config.max_attempts,
        max_iterations: config.max_iterations,
        git: &config.git,
        cleanup: CleanupPolicy {
            on_success: config.cleanup.effective(root),
        },
        checks: &config.checks,
    })
}
