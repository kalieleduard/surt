use super::driver::Driver;
use super::outcome::Progress;
use crate::project::{Error, Refusal};
use crate::runtime::AgentRuntime;

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn guard(&self) -> Result<Option<Refusal>, Error> {
        let store = self.driving.store;
        let recorded = self.working_copy.record(store, self.driving.actor).await;
        let scanned = match refused(recorded)? {
            Ok(scanned) => scanned,
            Err(refusal) => return Ok(Some(refusal)),
        };
        for edit in scanned.recorded {
            self.report(Progress::EditRecorded {
                artifact: edit.artifact,
                revision: edit.revision,
            });
        }
        for file in scanned.restored {
            self.report(Progress::FileRestored {
                artifact: file.artifact,
                path: file.path,
                revision: file.revision,
            });
        }
        let unapproved = self.working_copy.unapproved(store).await?;
        Ok(unapproved.map(|unapproved| self.working_copy.refuse(unapproved)))
    }
}

pub(super) fn refused<T>(result: Result<T, Error>) -> Result<Result<T, Refusal>, Error> {
    match result {
        Ok(value) => Ok(Ok(value)),
        Err(Error::Refused(refusal)) => Ok(Err(refusal)),
        Err(other) => Err(other),
    }
}
