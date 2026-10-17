use super::RunId;
use crate::store::StoreError;
use crate::workflow::{FreezeError, InvalidDefinition};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("refused: {0:?}")]
    Refused(Refusal),
    #[error(transparent)]
    Internal(#[from] InternalError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    ActiveRunExists { run: RunId },
    InvalidDefinition(Box<InvalidDefinition>),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct InternalError(InternalSource);

#[derive(Debug, thiserror::Error)]
enum InternalSource {
    #[error(transparent)]
    Store(StoreError),
    #[error(transparent)]
    Freeze(FreezeError),
}

impl From<StoreError> for Error {
    fn from(error: StoreError) -> Error {
        Error::Internal(InternalError(InternalSource::Store(error)))
    }
}

impl From<FreezeError> for Error {
    fn from(error: FreezeError) -> Error {
        match error {
            FreezeError::Invalid(invalid) => Error::Refused(Refusal::InvalidDefinition(invalid)),
            canonical => Error::Internal(InternalError(InternalSource::Freeze(canonical))),
        }
    }
}
