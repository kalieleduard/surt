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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Running,
    AwaitingApproval,
    Blocked,
    Interrupted,
    Succeeded,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub states: Vec<RunState>,
    pub awaiting: Option<AwaitedRevision>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownRun {
    pub run: u32,
    pub states: Vec<RunState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AwaitedRevision {
    pub artifact: String,
    pub revision: u32,
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
    DriverLive {
        subject: String,
        pid: Option<u32>,
        since: Option<i64>,
    },
    NoGateWaiting {
        subject: String,
    },
    StaleRevision {
        subject: String,
        artifact: String,
        expected: u32,
        latest: u32,
    },
    UnapprovedRevision {
        subject: String,
        artifact: String,
        revision: u32,
    },
    PinnedEdit {
        subject: String,
        artifact: String,
        revision: u32,
        files: Vec<String>,
    },
    InvalidEdit {
        subject: String,
        artifact: String,
        problem: String,
        files: Vec<String>,
    },
    AgentOverrides {
        path: PathBuf,
    },
    UnknownSubject {
        reference: String,
    },
    AmbiguousSubject {
        reference: String,
        candidates: Vec<Candidate>,
    },
    UnknownArtifact {
        subject: String,
        artifact: String,
        known: Vec<String>,
    },
    UnknownRevision {
        subject: String,
        artifact: String,
        revision: u32,
        latest: u32,
    },
    UnknownRun {
        subject: String,
        run: u32,
        runs: Vec<KnownRun>,
    },
    UnknownItem {
        subject: String,
        item: String,
        known: Vec<String>,
    },
    InvalidDefinition(Box<InvalidDefinition>),
    IntegrationBranchMoved {
        subject: String,
        branch: String,
        expected: String,
        found: Option<String>,
    },
    IntegrationBranchCheckedOut {
        subject: String,
        branch: String,
        path: PathBuf,
    },
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
    pub(crate) fn internal(error: impl StdError + Send + Sync + 'static) -> Error {
        Error::Internal(InternalError(Box::new(error)))
    }

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
