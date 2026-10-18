#[cfg(feature = "test-support")]
use std::pin::Pin;
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
pub(crate) type Interleaved = Pin<Box<dyn Future<Output = ()> + Send>>;

#[cfg(feature = "test-support")]
#[derive(Clone, Default)]
pub(crate) struct Faults {
    armed: Arc<Mutex<Option<Checkpoint>>>,
    before_purge: Arc<Mutex<Option<Interleaved>>>,
}

#[cfg(feature = "test-support")]
impl Faults {
    pub(crate) fn arm(&self, checkpoint: Checkpoint) {
        *self.armed() = Some(checkpoint);
    }

    pub(crate) fn interleave_before_purge(&self, work: Interleaved) {
        *lock(&self.before_purge) = Some(work);
    }

    pub(crate) async fn before_purge(&self) {
        let work = lock(&self.before_purge).take();
        if let Some(work) = work {
            work.await;
        }
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
        lock(&self.armed)
    }
}

#[cfg(feature = "test-support")]
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
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
