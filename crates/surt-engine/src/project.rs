mod base_merge;
mod blocked;
mod cancelling;
mod config;
mod create_run;
mod deleting;
mod delivery;
mod discovery;
mod driving;
mod error;
mod execution_state;
mod gate_status;
mod gates;
mod init;
mod inspect;
mod interrupted_merge;
mod layout;
mod logs;
mod naming;
mod repository;
mod resolving;
mod restoring;
mod show;
mod status;
mod stopping;
mod subjects;

use std::path::Path;
use std::time::Duration;

pub use crate::artifacts::working_copy::restoring::{Change, FileChange};
pub use crate::cleanup::{Cleanup, Kept, Left};
pub use crate::drive::cancel::Cancellation;
pub use crate::drive::graph::BlockReason;
pub use crate::drive::{
    ActivityPause, ActivitySnapshot, BaseMerge, DriveOutcome, Gate, NotMerged, Progress,
    ProgressSink, RunningStep, StepLabel,
};
pub use crate::git::GitVersion;
pub use crate::operations::checks::REPORT_SCHEMA as CHECK_REPORT_SCHEMA;
pub use crate::operations::{BASE_SCHEMA, CONFLICT_SCHEMA, INTEGRATION_SCHEMA};
pub use crate::platform::{console_outputs_utf8, stderr_columns};
pub use crate::runtime::LogFiles;
pub use crate::workflow::{
    AgentAssets, Catalog, DefinitionFile, DeliveryCheck, InvalidDefinition, InvalidOverride,
    OnSuccess, Problem, StepField, StepUse, WorkflowStep,
};
pub use base_merge::BaseMergeReport;
pub use blocked::{BlockedStep, LastFailure};
pub use cancelling::CancelImpact;
pub use config::{Check, CleanupConfig, Config, GitConfig, GlobalConfig, MergeIntoBase};
pub(crate) use config::{live_max_concurrent_agents, parse_timeout};
pub use create_run::{ArtifactFile, CreatedRun, InitialArtifact, LocalFiles, NewRun, RunId};
pub use deleting::{BranchFate, DeleteImpact, DeletedRun, Deletion, Directory};
pub use delivery::IntegrationBranch;
pub use error::{
    AwaitedRevision, Candidate, Error, ExecutionCandidate, InternalError, KnownRun, Refusal,
    RunState,
};
pub use execution_state::ExecutionState;
pub use gate_status::GateStatus;
pub use gates::{Decided, RevisionOrigin};
pub use init::{FileState, InitFile, Initialized};
pub use inspect::{
    AgentProfile, ExecutionDetail, ExecutionFailure, ExecutionInput, InspectRequest, Performer,
    RuntimeInfo, Timeline, TimelineEvent, Usage,
};
pub use interrupted_merge::InterruptedMerge;
pub use logs::{LogRequest, LogSource, Logs};
pub use naming::Slug;
pub use repository::Commit;
pub use resolving::{Reference, RunSelector, Subject};
pub use restoring::RestoreImpact;
pub use show::{ArtifactSummary, RevisionApproval, ShowRequest, Shown, ShownFile};
pub use status::{
    AgentSlots, CleanupState, Conflict, Driver, GateView, ItemState, ItemStatus, LaunchPause,
    PendingIntegration, RevisionSummary, RunStatus, StatusScope, Unattended,
};
pub use stopping::StopRequest;
pub use subjects::FreeName;

#[cfg(feature = "test-support")]
pub use crate::store::Checkpoint;
#[cfg(feature = "test-support")]
use crate::store::Faults;
use crate::store::Store;
pub(crate) use discovery::SurtHome;
pub(crate) use layout::Layout;

pub struct Project {
    layout: Layout,
    id: String,
    config: Config,
    global: GlobalConfig,
    catalog: Catalog,
    home: SurtHome,
    access: Access,
    #[cfg(feature = "test-support")]
    faults: Faults,
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
        Self::open_with(cwd, &SurtHome::from_env(), catalog, Access::ReadWrite).await
    }

    pub async fn open_read_only(cwd: &Path) -> Result<Project, Error> {
        Self::open_with(cwd, &SurtHome::from_env(), Catalog::new(), Access::ReadOnly).await
    }

    async fn open_with(
        cwd: &Path,
        home: &SurtHome,
        catalog: Catalog,
        access: Access,
    ) -> Result<Project, Error> {
        let layout = discovery::find_project(cwd, home).await?;
        let config = config::load(&layout.config())?;
        let global = home
            .config()
            .map_or_else(|| Ok(GlobalConfig::default()), config::read)?;
        let store = open(&layout, access).await?;
        let id = store.project_id().await;
        store.close().await;
        Ok(Project {
            layout,
            id: id?,
            config,
            global,
            catalog,
            home: home.clone(),
            access,
            #[cfg(feature = "test-support")]
            faults: Faults::default(),
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

    pub fn stop_grace_period(&self) -> Duration {
        self.global.stop_grace_period
    }

    #[cfg(feature = "test-support")]
    pub fn crash_after(&self, checkpoint: Checkpoint) {
        self.faults.arm(checkpoint);
    }

    #[cfg(feature = "test-support")]
    pub fn interleave_before_purge(&self, work: impl Future<Output = ()> + Send + 'static) {
        self.faults.interleave_before_purge(Box::pin(work));
    }

    async fn store(&self) -> Result<Store, Error> {
        let store = open_store(&self.layout).await?;
        #[cfg(feature = "test-support")]
        let store = store.with_faults(self.faults.clone());
        Ok(store)
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
