use std::path::PathBuf;

use super::outcome::Progress;
use crate::project::live_max_concurrent_agents;

pub(super) struct LiveLimit {
    path: PathBuf,
    limit: usize,
    warned: bool,
}

impl LiveLimit {
    pub(super) fn new(path: PathBuf, limit: usize) -> LiveLimit {
        LiveLimit {
            path,
            limit,
            warned: false,
        }
    }

    pub(super) fn reread(&mut self, report: impl FnOnce(Progress)) -> usize {
        match live_max_concurrent_agents(&self.path) {
            Ok(limit) => {
                self.warned = false;
                if let Some(limit) = limit {
                    self.limit = usize::try_from(limit.get()).unwrap_or(usize::MAX);
                }
            }
            Err(unreadable) if !self.warned => {
                self.warned = true;
                report(Progress::ConfigInvalid {
                    path: self.path.clone(),
                    line: unreadable.line,
                    reason: unreadable.reason,
                    limit: self.limit,
                });
            }
            Err(_) => {}
        }
        self.limit
    }
}
