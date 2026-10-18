use super::decide::RunEnd;
use super::driver::Driving;
use super::outcome::{DriveOutcome, Progress, ProgressSink};
use crate::cleanup;
use crate::project::Error;

pub(super) async fn ended(
    driving: &Driving<'_>,
    progress: &ProgressSink,
) -> Result<Option<DriveOutcome>, Error> {
    let Some(ending) = driving.store.ending(driving.root).await? else {
        return Ok(None);
    };
    if let Some(cleanup) = cleanup::clean_up(driving.store, driving.layout, driving.root).await? {
        let _ = progress.send(Progress::CleanedUp(cleanup));
    }
    Ok(Some(match ending.end {
        RunEnd::Succeeded => DriveOutcome::Succeeded,
        RunEnd::Cancelled => DriveOutcome::Cancelled,
    }))
}
