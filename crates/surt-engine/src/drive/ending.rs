use super::decide::RunEnd;
use super::driver::Driving;
use super::outcome::{DriveOutcome, Progress, ProgressSink};
use crate::cleanup;
use crate::project::Error;

pub(super) async fn ended_before(
    driving: &Driving<'_>,
    progress: &ProgressSink,
) -> Result<Option<DriveOutcome>, Error> {
    Ok(ended(driving, progress).await?.map(|end| match end {
        RunEnd::Succeeded => DriveOutcome::Succeeded,
        RunEnd::Cancelled => DriveOutcome::AlreadyCancelled,
    }))
}

pub(super) async fn ended_while_driving(
    driving: &Driving<'_>,
    progress: &ProgressSink,
) -> Result<Option<DriveOutcome>, Error> {
    Ok(ended(driving, progress).await?.map(|end| match end {
        RunEnd::Succeeded => DriveOutcome::Succeeded,
        RunEnd::Cancelled => DriveOutcome::Cancelled,
    }))
}

async fn ended(driving: &Driving<'_>, progress: &ProgressSink) -> Result<Option<RunEnd>, Error> {
    let Some(ending) = driving.store.ending(driving.root).await? else {
        return Ok(None);
    };
    if let Some(cleanup) = cleanup::clean_up(driving.store, driving.layout, driving.root).await? {
        let _ = progress.send(Progress::CleanedUp(cleanup));
    }
    Ok(Some(ending.end))
}
