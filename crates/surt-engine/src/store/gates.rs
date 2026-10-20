use sqlx::FromRow;

use super::columns::number;
use super::{Store, StoreError};

const INPUT: &str = "input";

#[derive(Clone, Debug, FromRow)]
pub(crate) struct WaitingGate {
    pub(crate) id: String,
    pub(crate) run: String,
    pub(crate) key: String,
    pub(crate) kind: String,
    pub(crate) definition: String,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct GateRow {
    pub(crate) kind: String,
    pub(crate) state: String,
    pub(crate) outcome: Option<String>,
    pub(crate) definition: String,
    decided: Option<i64>,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct GateViewRow {
    pub(crate) key: String,
    pub(crate) kind: String,
    pub(crate) state: String,
    pub(crate) outcome: Option<String>,
    pub(crate) definition: String,
    pub(crate) decided: Option<i64>,
    pub(crate) actor: Option<String>,
    pub(crate) decided_at: Option<i64>,
    pub(crate) round: Option<i64>,
}

impl WaitingGate {
    pub(crate) fn is_input(&self) -> bool {
        self.kind == INPUT
    }
}

impl GateRow {
    pub(crate) fn is_input(&self) -> bool {
        self.kind == INPUT
    }

    pub(crate) fn decided(&self) -> Option<u32> {
        self.decided.map(number)
    }
}

impl GateViewRow {
    pub(crate) fn is_input(&self) -> bool {
        self.kind == INPUT
    }
}

impl Store {
    pub(crate) async fn latest_gate(
        &self,
        root: &str,
        key: &str,
    ) -> Result<Option<GateRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT s.kind, s.state, s.outcome, w.definition_sha256 AS definition, \
             coalesce(r.number, json_extract(e.data, '$.revision')) AS decided \
             FROM step_run s JOIN workflow_run w ON w.id = s.workflow_run_id \
             LEFT JOIN approval a ON a.gate_step_run_id = s.id \
             LEFT JOIN artifact_revision r ON r.id = a.revision_id \
             LEFT JOIN event e ON e.step_run_id = s.id AND e.kind = 'answer.recorded' \
             WHERE s.workflow_run_id = ? AND s.step_key = ? AND s.kind IN ('approval', 'input') \
             ORDER BY s.iteration DESC, s.created_at DESC LIMIT 1",
        )
        .bind(root)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub(crate) async fn waiting_gates(&self, root: &str) -> Result<Vec<WaitingGate>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT s.id, s.workflow_run_id AS run, s.step_key AS key, s.kind, \
             w.definition_sha256 AS definition \
             FROM step_run s JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.root_run_id = ? AND s.kind IN ('approval', 'input') AND s.state = 'running' \
             ORDER BY s.created_at, s.id",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn gate_views(&self, root: &str) -> Result<Vec<GateViewRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT s.step_key AS key, s.kind, s.state, s.outcome, w.definition_sha256 AS definition, \
             coalesce(r.number, json_extract(e.data, '$.revision')) AS decided, \
             coalesce(a.actor, json_extract(e.data, '$.actor')) AS actor, \
             coalesce(a.decided_at, e.at) AS decided_at, json_extract(e.data, '$.round') AS round \
             FROM step_run s JOIN workflow_run w ON w.id = s.workflow_run_id \
             LEFT JOIN approval a ON a.gate_step_run_id = s.id \
             LEFT JOIN artifact_revision r ON r.id = a.revision_id \
             LEFT JOIN event e ON e.step_run_id = s.id AND e.kind = 'answer.recorded' \
             WHERE s.workflow_run_id = ?1 AND s.kind IN ('approval', 'input') AND s.id = ( \
               SELECT g.id FROM step_run g WHERE g.workflow_run_id = ?1 AND g.step_key = s.step_key \
               ORDER BY g.iteration DESC, g.created_at DESC, g.rowid DESC LIMIT 1) \
             ORDER BY s.created_at, s.rowid",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?)
    }
}
