use std::fs;
use std::path::PathBuf;

use super::{Error, Layout, Refusal};
use crate::platform::{self, ExclusiveLock};
use crate::store::SubjectRun;

pub(super) struct RunLocks {
    held: Vec<(PathBuf, ExclusiveLock)>,
}

impl RunLocks {
    pub(super) async fn take(
        layout: &Layout,
        subject: &str,
        runs: &[SubjectRun],
    ) -> Result<RunLocks, Error> {
        let paths: Vec<PathBuf> = runs.iter().map(|run| layout.run_lock(&run.id)).collect();
        let held = tokio::task::spawn_blocking(move || try_all(paths))
            .await
            .map_err(Error::internal)??;
        match held {
            Some(held) => Ok(RunLocks { held }),
            None => Err(Refusal::RunLockBusy {
                subject: subject.to_owned(),
            }
            .into()),
        }
    }

    pub(super) fn release(self) {
        for (path, lock) in self.held {
            drop(lock);
            if let Err(error) = fs::remove_file(&path) {
                tracing::debug!(path = %path.display(), %error, "the run lock file stays");
            }
        }
    }
}

fn try_all(paths: Vec<PathBuf>) -> Result<Option<Vec<(PathBuf, ExclusiveLock)>>, Error> {
    let mut held = Vec::new();
    for path in paths {
        match platform::try_lock_exclusive(&path) {
            Ok(Some(lock)) => held.push((path, lock)),
            Ok(None) => return Ok(None),
            Err(error) => return Err(Error::io("lock", &path, error)),
        }
    }
    Ok(Some(held))
}
