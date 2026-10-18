use super::frozen::CorruptSnapshot;
use crate::git::GitError;
use crate::project::Error;
use crate::runtime::FailureClass;
use crate::store::{ExecutionEnd, StoreError};

#[derive(Debug)]
pub(super) struct Failure {
    class: FailureClass,
    code: &'static str,
    message: String,
}

pub(super) enum OperationError {
    Failed(Failure),
    Fatal(Error),
}

impl Failure {
    pub(super) fn not_retryable(code: &'static str, message: String) -> Failure {
        Failure {
            class: FailureClass::NotRetryable,
            code,
            message,
        }
    }

    pub(super) fn retryable(code: &'static str, message: String) -> Failure {
        Failure {
            class: FailureClass::Retryable,
            code,
            message,
        }
    }

    pub(super) fn into_end(self) -> ExecutionEnd {
        ExecutionEnd::Failed {
            class: self.class,
            code: self.code.to_owned(),
            message: self.message,
            resets_at: None,
        }
    }
}

impl From<Failure> for OperationError {
    fn from(failure: Failure) -> OperationError {
        OperationError::Failed(failure)
    }
}

impl From<GitError> for OperationError {
    fn from(error: GitError) -> OperationError {
        OperationError::Failed(Failure::from(error))
    }
}

impl From<GitError> for Failure {
    fn from(error: GitError) -> Failure {
        let code = match &error {
            GitError::LockHeld { .. } => "git_lock_held",
            _ => "git_failed",
        };
        Failure::retryable(code, error.to_string())
    }
}

impl From<StoreError> for OperationError {
    fn from(error: StoreError) -> OperationError {
        OperationError::Fatal(error.into())
    }
}

impl From<Error> for OperationError {
    fn from(error: Error) -> OperationError {
        OperationError::Fatal(error)
    }
}

impl From<CorruptSnapshot> for OperationError {
    fn from(error: CorruptSnapshot) -> OperationError {
        OperationError::Fatal(error.into())
    }
}
