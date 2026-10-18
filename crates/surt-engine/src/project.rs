mod blocked;
mod config;
mod create_run;
mod delivery;
mod discovery;
mod driving;
mod error;
mod gate_status;
mod gates;
mod init;
mod layout;
mod naming;
mod repository;
mod resolving;
mod subjects;

use std::path::Path;

pub use crate::checks::REPORT_SCHEMA as CHECK_REPORT_SCHEMA;
pub use crate::drive::graph::BlockReason;
pub use crate::drive::{DriveOutcome, Gate, Progress, ProgressSink, StepLabel};
pub use crate::git::GitVersion;
pub use crate::operations::{BASE_SCHEMA, CONFLICT_SCHEMA, INTEGRATION_SCHEMA};
pub use crate::workflow::{
    AgentAssets, Catalog, DefinitionFile, DeliveryCheck, InvalidDefinition, Problem, StepField,
};
pub use blocked::{BlockedStep, LastFailure};
pub(crate) use config::parse_timeout;
pub use config::{Check, Config, GitConfig, MergeIntoBase};
pub use create_run::{ArtifactFile, InitialArtifact, NewRun, RunId};
pub use delivery::IntegrationBranch;
pub use error::{Error, InternalError, Refusal};
pub use gate_status::GateStatus;
pub use gates::{Decided, RevisionOrigin};
pub use init::{FileState, InitFile, Initialized};
pub use naming::Slug;
pub use repository::Commit;
pub use resolving::Subject;
pub use subjects::FreeName;

use crate::store::Store;
pub(crate) use discovery::SurtHome;
pub(crate) use layout::Layout;

pub struct Project {
    layout: Layout,
    id: String,
    config: Config,
    catalog: Catalog,
    home: SurtHome,
}

impl Project {
    pub async fn init(cwd: &Path) -> Result<Initialized, Error> {
        init::run(cwd, &SurtHome::from_env()).await
    }

    pub async fn open(cwd: &Path, catalog: Catalog) -> Result<Project, Error> {
        Self::open_in(cwd, &SurtHome::from_env(), catalog).await
    }

    async fn open_in(cwd: &Path, home: &SurtHome, catalog: Catalog) -> Result<Project, Error> {
        let layout = discovery::find_project(cwd, home).await?;
        let config = config::load(&layout.config())?;
        let store = open_store(&layout).await?;
        let id = store.project_id().await?;
        store.close().await;
        Ok(Project {
            layout,
            id,
            config,
            catalog,
            home: home.clone(),
        })
    }

    pub fn root(&self) -> &Path {
        self.layout.root()
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    fn refuse_agent_overrides(&self) -> Result<(), Error> {
        let agents = self.layout.agents();
        if agents.exists() {
            return Err(Refusal::AgentOverrides { path: agents }.into());
        }
        Ok(())
    }
}

async fn open_store(layout: &Layout) -> Result<Store, Error> {
    Ok(Store::open(
        &layout.database(),
        &layout.database_lock(),
        &layout.project_name(),
    )
    .await?)
}

#[cfg(test)]
mod tests;
