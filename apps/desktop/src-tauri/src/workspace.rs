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

    fn root(&self) -> Option<PathBuf> {
        self.root.lock().ok()?.clone()
    }
}

#[cfg(test)]
mod tests;
