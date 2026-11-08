use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Serialize, Serializer};
use surt_engine::project::{Error, Project, Refusal};

const PROJECT_VARIABLE: &str = "SURT_PROJECT";

pub struct Workspace {
    root: Mutex<Option<PathBuf>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Failure {
    #[error("no Surt project is open")]
    NoProject,
    #[error("{} is not a Surt project: run surt init there first", .0.display())]
    NotAProject(PathBuf),
    #[error("{0} has no active run")]
    NoActiveRun(String),
    #[error("{0} is already being driven from this window")]
    AlreadyDriving(String),
    #[error("cannot derive a subject name from {0:?}")]
    NoSlug(String),
    #[error("{0} is not an agent or workflow file")]
    OutsideDefinitions(String),
    #[error("{0} cannot be deleted: only references and schemas can")]
    NotRemovable(String),
    #[error("cannot access {0}: {1}")]
    Io(String, String),
    #[error("{0} is not a worktree of this repository")]
    UnknownWorktree(String),
    #[error(
        "runs are being driven from this window: stop them before switching the main checkout's branch"
    )]
    DrivingInMain,
    #[error("{0}")]
    Git(String),
    #[error("no script named {0} in .surt/config.toml")]
    UnknownScript(String),
    #[error("{0} is already running")]
    ScriptRunning(String),
    #[cfg(feature = "test-support")]
    #[error("{0}")]
    Runtime(String),
    #[error("{0}")]
    Engine(Error),
}

impl From<Error> for Failure {
    fn from(error: Error) -> Failure {
        match error {
            Error::Refused(Refusal::NotAProject { root }) => Failure::NotAProject(root),
            other => Failure::Engine(other),
        }
    }
}

impl Serialize for Failure {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Workspace {
    pub fn from_env() -> Workspace {
        Workspace {
            root: Mutex::new(std::env::var_os(PROJECT_VARIABLE).map(PathBuf::from)),
        }
    }

    pub fn has_project(&self) -> bool {
        self.root().is_some()
    }

    pub fn select(&self, root: PathBuf) {
        if let Ok(mut current) = self.root.lock() {
            *current = Some(root);
        }
    }

    pub async fn project(&self) -> Result<Project, Failure> {
        let root = self.root().ok_or(Failure::NoProject)?;
        Ok(Project::open_read_only(&root).await?)
    }

    pub async fn writable(&self) -> Result<Project, Failure> {
        let root = self.root().ok_or(Failure::NoProject)?;
        Ok(Project::open(&root).await?)
    }

    fn root(&self) -> Option<PathBuf> {
        self.root.lock().ok()?.clone()
    }
}

#[cfg(test)]
mod tests;
