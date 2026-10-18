use std::path::Path;

use serde_json::json;

use super::super::driver::Driver;
use super::super::failure::OperationError;
use super::super::graph::ExecutionState;
use super::super::load::label;
use super::super::outcome::Progress;
use crate::reconcile::worktree::{Located, Prepared, Repair};
use crate::runtime::AgentRuntime;
use crate::store::{PreviousExecution, StartedExecution, StepRow};

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn record_preparation(
        &self,
        row: &StepRow,
        started: &StartedExecution,
        path: &Path,
        previous: Option<&PreviousExecution>,
        prepared: Prepared,
    ) -> Result<(), OperationError> {
        let store = self.driving.store;
        let repair = match prepared.located {
            Located::Repaired(repair) => Some(repair),
            Located::Added if previous.is_some() => Some(Repair::Missing),
            Located::Added | Located::Present => None,
        };
        if let Some(repair) = repair {
            let data = json!({ "path": path, "repair": repair_name(&repair) });
            store
                .record_execution_event(&started.id, "worktree.repaired", &data)
                .await?;
            self.report(Progress::WorktreeRepaired {
                step: label(row),
                path: path.to_path_buf(),
            });
        }
        let Some(salvage) = prepared.salvage else {
            return Ok(());
        };
        let data = json!({ "ref": salvage.reference });
        store
            .record_execution_event(&started.id, "worktree.salvaged", &data)
            .await?;
        if salvage.off_target || previous.is_some_and(ended_badly) {
            self.report(Progress::Salvaged {
                step: label(row),
                reference: salvage.reference,
            });
        }
        Ok(())
    }
}

fn ended_badly(previous: &PreviousExecution) -> bool {
    matches!(
        previous.state,
        ExecutionState::Failed(_) | ExecutionState::Interrupted
    )
}

fn repair_name(repair: &Repair) -> &'static str {
    match repair {
        Repair::HalfCreated => "half_created",
        Repair::Missing => "missing",
        Repair::Orphaned { .. } => "orphaned",
    }
}
