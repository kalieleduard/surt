use std::collections::{BTreeMap, BTreeSet};

use sqlx::FromRow;

use super::{Store, StoreError};

#[derive(Clone, Debug)]
pub(crate) struct PreviousConversation {
    pub(crate) execution: String,
    pub(crate) session_id: String,
    pub(crate) inputs: BTreeMap<String, String>,
    pub(crate) produced: BTreeSet<String>,
}

#[derive(FromRow)]
struct LatestRow {
    id: String,
    session_id: String,
}

impl Store {
    pub(crate) async fn previous_conversation(
        &self,
        step: &str,
    ) -> Result<Option<PreviousConversation>, StoreError> {
        let mut snapshot = self.pool.begin().await?;
        let latest: Option<LatestRow> = sqlx::query_as(
            "SELECT e.id, json_extract(e.runtime_info, '$.session_id') AS session_id \
             FROM step_run current JOIN step_run s ON s.workflow_run_id = current.workflow_run_id \
             AND s.loop_step_run_id = current.loop_step_run_id AND s.step_key = current.step_key \
             JOIN execution e ON e.step_run_id = s.id \
             WHERE current.id = ? AND e.state = 'succeeded' \
             AND json_extract(e.runtime_info, '$.session_id') IS NOT NULL \
             ORDER BY s.iteration DESC, e.attempt DESC LIMIT 1",
        )
        .bind(step)
        .fetch_optional(&mut *snapshot)
        .await?;
        let Some(latest) = latest else {
            return Ok(None);
        };
        let inputs: Vec<(String, String)> = sqlx::query_as(
            "SELECT input_name, revision_id FROM execution_input WHERE execution_id = ?",
        )
        .bind(&latest.id)
        .fetch_all(&mut *snapshot)
        .await?;
        let produced: Vec<(String,)> =
            sqlx::query_as("SELECT id FROM artifact_revision WHERE execution_id = ?")
                .bind(&latest.id)
                .fetch_all(&mut *snapshot)
                .await?;
        snapshot.commit().await?;
        Ok(Some(PreviousConversation {
            execution: latest.id,
            session_id: latest.session_id,
            inputs: inputs.into_iter().collect(),
            produced: produced.into_iter().map(|(id,)| id).collect(),
        }))
    }
}
