use super::{TaskState, TaskStatus};
use crate::project::Error;
use crate::store::{Store, TaskRow};

pub(super) async fn statuses(store: &Store, root: &str) -> Result<Vec<TaskStatus>, Error> {
    Ok(store
        .tasks(root)
        .await?
        .into_iter()
        .map(|row| TaskStatus {
            state: state(&row),
            iteration: row.iteration.and_then(|value| u32::try_from(value).ok()),
            attempt: row.attempt.and_then(|value| u32::try_from(value).ok()),
            since: row.started_at,
            merged_as: row.merged_as,
            step: row.step_key,
            key: row.key,
        })
        .collect())
}

fn state(row: &TaskRow) -> TaskState {
    match (
        row.run_state.as_str(),
        row.step_state.as_deref(),
        row.execution_state.as_deref(),
    ) {
        ("succeeded", _, _) => TaskState::Succeeded,
        ("cancelled", _, _) => TaskState::Cancelled,
        (_, Some("blocked"), _) => TaskState::Blocked,
        (_, Some("running"), Some("running")) => TaskState::Running,
        (_, Some("running"), Some("queued")) => TaskState::Queued,
        (_, None | Some("pending"), _) => TaskState::Pending,
        _ => TaskState::Interrupted,
    }
}
