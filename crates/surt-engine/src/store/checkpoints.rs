#[cfg(feature = "test-support")]
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::{Store, StoreError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checkpoint {
    RunCreated,
    IterationOrExpansion,
    ExecutionStarted,
    ProcessSpawned,
    Intention,
    ExecutionAccepted,
    WorkingCopyWritten,
    ExecutionEnded,
}

#[cfg(feature = "test-support")]
#[derive(Clone, Debug, Default)]
pub(crate) struct Faults {
    armed: Arc<Mutex<Option<Checkpoint>>>,
}

#[cfg(feature = "test-support")]
impl Faults {
    pub(crate) fn arm(&self, checkpoint: Checkpoint) {
        *self.armed() = Some(checkpoint);
    }

    fn trip(&self, checkpoint: Checkpoint) -> Result<(), StoreError> {
        let mut armed = self.armed();
        if *armed != Some(checkpoint) {
            return Ok(());
        }
        *armed = None;
        Err(StoreError::Crashed(checkpoint))
    }

    fn armed(&self) -> MutexGuard<'_, Option<Checkpoint>> {
        self.armed.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(feature = "test-support")]
impl Store {
    pub(crate) fn with_faults(mut self, faults: Faults) -> Store {
        self.faults = faults;
        self
    }

    pub(super) fn reached(&self, checkpoint: Checkpoint) -> Result<(), StoreError> {
        self.faults.trip(checkpoint)
    }
}

#[cfg(not(feature = "test-support"))]
impl Store {
    pub(super) fn reached(&self, _checkpoint: Checkpoint) -> Result<(), StoreError> {
        Ok(())
    }
}
