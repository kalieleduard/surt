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
mod show;
mod status;
mod subjects;

use std::path::Path;

pub use crate::checks::REPORT_SCHEMA as CHECK_REPORT_SCHEMA;
pub use crate::cleanup::{Cleanup, Kept, Left};
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
pub use show::{ArtifactSummary, ShowRequest, Shown};
pub use status::{
    CleanupState, Driver, GateView, PendingIntegration, RevisionSummary, RunState, RunStatus,
    StatusScope, TaskState, TaskStatus, Unattended,
};
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
    access: Access,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Access {
    ReadWrite,
    ReadOnly,
}

impl Project {
    pub async fn init(cwd: &Path) -> Result<Initialized, Error> {
        init::run(cwd, &SurtHome::from_env()).await
    }

    pub async fn open(cwd: &Path, catalog: Catalog) -> Result<Project, Error> {
        Self::open_in(cwd, &SurtHome::from_env(), catalog).await
    }

    pub async fn open_read_only(cwd: &Path) -> Result<Project, Error> {
        Self::open_with(cwd, &SurtHome::from_env(), Catalog::new(), Access::ReadOnly).await
    }

    async fn open_in(cwd: &Path, home: &SurtHome, catalog: Catalog) -> Result<Project, Error> {
        Self::open_with(cwd, home, catalog, Access::ReadWrite).await
    }

    async fn open_with(
        cwd: &Path,
        home: &SurtHome,
        catalog: Catalog,
        access: Access,
    ) -> Result<Project, Error> {
        let layout = discovery::find_project(cwd, home).await?;
        let config = config::load(&layout.config())?;
        let store = open(&layout, access).await?;
        let id = store.project_id().await;
        store.close().await;
        Ok(Project {
            layout,
            id: id?,
            config,
            catalog,
            home: home.clone(),
            access,
        })
    }

    async fn reader(&self) -> Result<Store, Error> {
        open(&self.layout, self.access).await
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

async fn open(layout: &Layout, access: Access) -> Result<Store, Error> {
    match access {
        Access::ReadWrite => open_store(layout).await,
        Access::ReadOnly => Ok(Store::open_read_only(&layout.database()).await?),
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
