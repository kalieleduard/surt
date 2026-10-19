mod inputs;
mod previous;

use std::num::NonZeroU32;

use serde::Serialize;

use super::definitions::ProjectDefinitions;
use super::{Check, Commit, Config, Error, GitConfig, MergeIntoBase, Project, Refusal};
use crate::cleanup::clear_kept_working_copy;
pub use crate::drive::graph::RunId;
use crate::store::{NewInitialArtifact, NewRunRows, RunCreation, Store, unix_now};
use crate::workflow::{CleanupPolicy, FrozenRun, OnSuccess, freeze};

#[derive(Clone, Debug)]
pub struct NewRun {
    pub subject: String,
    pub workflow: String,
    pub inputs: Vec<(String, String)>,
    pub carried: Vec<String>,
    pub merge_into_base: Option<MergeIntoBase>,
    pub actor: String,
}

pub use inputs::{DeclaredInput, InputProblem};

#[derive(Clone, Debug)]
pub(crate) struct InitialArtifact {
    pub(crate) name: String,
    pub(crate) files: Vec<ArtifactFile>,
}

#[derive(Clone, Debug)]
pub(crate) struct ArtifactFile {
    pub(crate) rel_path: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Serialize)]
struct FrozenConfig<'a> {
    max_attempts: NonZeroU32,
    max_iterations: NonZeroU32,
    git: GitConfig,
    cleanup: CleanupPolicy,
    checks: &'a [Check],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatedRun {
    pub run: RunId,
    pub merge_into_base: MergeIntoBase,
    pub git_refs: Vec<(String, Commit)>,
}

impl Project {
    pub async fn create_run(&self, new: NewRun) -> Result<CreatedRun, Error> {
        let merge_into_base = new
            .merge_into_base
            .unwrap_or(self.config.git.merge_into_base);
        let store = self.store().await?;
        let created = self.create_with(&store, new, merge_into_base).await;
        store.close().await;
        let (run, git_refs) = created?;
        Ok(CreatedRun {
            run,
            merge_into_base,
            git_refs,
        })
    }

    async fn create_with(
        &self,
        store: &Store,
        new: NewRun,
        merge_into_base: MergeIntoBase,
    ) -> Result<(RunId, Vec<(String, Commit)>), Error> {
        let checked = self.checked(store, new, merge_into_base).await?;
        if let Some(previous) = &checked.previous {
            clear_kept_working_copy(store, &self.layout, previous).await?;
        }
        let subject = checked.rows.subject.clone();
        match store.create_run(checked.rows).await? {
            RunCreation::Created(run) => Ok((RunId(run), checked.git_refs)),
            RunCreation::ActiveRunExists(_) => {
                Err(Error::Refused(Refusal::ActiveRunExists { subject }))
            }
        }
    }

    async fn checked(
        &self,
        store: &Store,
        new: NewRun,
        merge_into_base: MergeIntoBase,
    ) -> Result<Checked, Error> {
        let definitions = ProjectDefinitions::new(&self.layout);
        definitions.require(&new.workflow)?;
        let frozen = freeze(&definitions, &new.workflow, |root| {
            frozen_config(&self.config, merge_into_base, root)
        })?;
        let refuse = |problem| inputs::refused(&new.workflow, &frozen.inputs, problem);
        let supplied = inputs::check(&frozen.inputs, &new.inputs, &new.carried).map_err(refuse)?;
        let mut built = inputs::build(self, supplied, refuse).await?;
        let carried = previous::carried(store, &self.layout, &new.subject, &built.carried).await?;
        built.artifacts.extend(carried.artifacts);
        Ok(Checked {
            git_refs: built.git_refs,
            rows: rows(new, frozen, built.artifacts),
            previous: carried.previous,
        })
    }
}

struct Checked {
    rows: NewRunRows,
    git_refs: Vec<(String, Commit)>,
    previous: Option<RunId>,
}

fn rows(new: NewRun, frozen: FrozenRun, artifacts: Vec<InitialArtifact>) -> NewRunRows {
    NewRunRows {
        subject: new.subject,
        frozen,
        initial_artifacts: artifacts
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

fn frozen_config(
    config: &Config,
    merge_into_base: MergeIntoBase,
    root: OnSuccess,
) -> Result<String, toml::ser::Error> {
    toml::to_string(&FrozenConfig {
        max_attempts: config.max_attempts,
        max_iterations: config.max_iterations,
        git: GitConfig { merge_into_base },
        cleanup: CleanupPolicy {
            on_success: config.cleanup.effective(root),
        },
        checks: &config.checks,
    })
}
