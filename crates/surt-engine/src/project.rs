mod config;
mod discovery;
mod error;
mod init;
mod layout;

use std::path::Path;

pub use crate::git::GitVersion;
pub use config::{Check, Config, GitConfig, MergeIntoBase};
pub use error::{Error, InternalError, Refusal};
pub use init::{FileState, InitFile, Initialized};

use crate::store::Store;
use discovery::SurtHome;
use layout::Layout;

pub struct Project {
    layout: Layout,
    id: String,
    config: Config,
}

impl Project {
    pub async fn init(cwd: &Path) -> Result<Initialized, Error> {
        init::run(cwd, &SurtHome::from_env()).await
    }

    pub async fn open(cwd: &Path) -> Result<Project, Error> {
        Self::open_in(cwd, &SurtHome::from_env()).await
    }

    async fn open_in(cwd: &Path, home: &SurtHome) -> Result<Project, Error> {
        let layout = discovery::find_project(cwd, home).await?;
        let config = config::load(&layout.config())?;
        let store = Store::open(
            &layout.database(),
            &layout.database_lock(),
            &layout.project_name(),
        )
        .await?;
        let id = store.project_id().await?;
        store.close().await;
        Ok(Project { layout, id, config })
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
}

#[cfg(test)]
mod tests;
