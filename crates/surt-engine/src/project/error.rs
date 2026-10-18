use std::error::Error as StdError;
use std::io;
use std::path::{Path, PathBuf};

use crate::git::{GitError, GitVersion};
use crate::store::StoreError;
use crate::workflow::{FreezeError, InvalidDefinition};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("refused: {0:?}")]
    Refused(Refusal),
    #[error(transparent)]
    Internal(#[from] InternalError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    NotAProject {
        root: PathBuf,
    },
    InsideManagedWorktree {
        path: PathBuf,
        project: Option<PathBuf>,
    },
    NotAGitRepository {
        path: PathBuf,
    },
    BareRepository {
        path: PathBuf,
    },
    GitNotFound,
    GitTooOld {
        found: GitVersion,
        min: GitVersion,
    },
    NoGitIdentity,
    SchemaTooNew {
        db: i64,
        supported: i64,
    },
    InvalidConfig {
        path: PathBuf,
        line: Option<usize>,
        reason: String,
    },
    MergeIntoBaseUnsupported {
        path: PathBuf,
    },
    ActiveRunExists {
        subject: String,
    },
    InvalidDefinition(Box<InvalidDefinition>),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct InternalError(Box<dyn StdError + Send + Sync + 'static>);

#[derive(Debug, thiserror::Error)]
#[error("cannot {action} {}", path.display())]
struct IoFailure {
    action: &'static str,
    path: PathBuf,
    #[source]
    source: io::Error,
}

impl Error {
    pub(crate) fn io(action: &'static str, path: &Path, source: io::Error) -> Error {
        Error::Internal(InternalError(Box::new(IoFailure {
            action,
            path: path.to_path_buf(),
            source,
        })))
    }
}

impl From<Refusal> for Error {
    fn from(refusal: Refusal) -> Error {
        Error::Refused(refusal)
    }
}

impl From<StoreError> for Error {
    fn from(error: StoreError) -> Error {
        match error {
            StoreError::SchemaTooNew { db, supported } => {
                Error::Refused(Refusal::SchemaTooNew { db, supported })
            }
            other => Error::Internal(InternalError(Box::new(other))),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Error {
        Error::from(StoreError::from(error))
    }
}

impl From<GitError> for Error {
    fn from(error: GitError) -> Error {
        match error {
            GitError::NotFound => Error::Refused(Refusal::GitNotFound),
            other => Error::Internal(InternalError(Box::new(other))),
        }
    }
}

impl From<FreezeError> for Error {
    fn from(error: FreezeError) -> Error {
        match error {
            FreezeError::Invalid(invalid) => Error::Refused(Refusal::InvalidDefinition(invalid)),
            canonical => Error::Internal(InternalError(Box::new(canonical))),
        }
    }
}
