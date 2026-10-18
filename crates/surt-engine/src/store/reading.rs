use sqlx::FromRow;

use super::columns::number;
use super::{Store, StoreError};
use crate::platform::ProcessIdentity;

#[derive(Clone, Debug, FromRow)]
pub(crate) struct RootRunRow {
    pub(crate) id: String,
    pub(crate) subject: String,
    pub(crate) ordinal: i64,
    pub(crate) state: String,
    pub(crate) definition: String,
    pub(crate) cleaned: bool,
    driver_pid: Option<i64>,
    driver_start_time: Option<i64>,
    driver_boot_id: Option<String>,
    pub(crate) driver_since: Option<i64>,
    pub(crate) finished_at: Option<i64>,
    runs: i64,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct ItemRunRow {
    pub(crate) run: String,
    pub(crate) key: String,
    pub(crate) run_state: String,
    pub(crate) merged_as: Option<String>,
    pub(crate) step_key: Option<String>,
    pub(crate) step_kind: Option<String>,
    pub(crate) step_state: Option<String>,
    pub(crate) iteration: Option<i64>,
    pub(crate) attempt: Option<i64>,
    pub(crate) execution_state: Option<String>,
    pub(crate) started_at: Option<i64>,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct GateViewRow {
    pub(crate) key: String,
    pub(crate) state: String,
    pub(crate) outcome: Option<String>,
    pub(crate) definition: String,
    pub(crate) decided: Option<i64>,
    pub(crate) actor: Option<String>,
    pub(crate) decided_at: Option<i64>,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct ArtifactSummaryRow {
    pub(crate) name: String,
    pub(crate) revisions: i64,
    pub(crate) number: i64,
    pub(crate) origin: String,
    pub(crate) approved: bool,
    pub(crate) files: i64,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct SlotHoldersRow {
    driver_pid: Option<i64>,
    driver_start_time: Option<i64>,
    driver_boot_id: Option<String>,
    agents: i64,
}

impl SlotHoldersRow {
    pub(crate) fn driver(&self) -> Option<ProcessIdentity> {
        identity(
            self.driver_pid,
            self.driver_start_time,
            self.driver_boot_id.as_deref(),
        )
    }

    pub(crate) fn agents(&self) -> u32 {
        number(self.agents)
    }
}

fn identity(
    pid: Option<i64>,
    start_time: Option<i64>,
    boot_id: Option<&str>,
) -> Option<ProcessIdentity> {
    Some(ProcessIdentity {
        pid: u32::try_from(pid?).ok()?,
        start_time: start_time?,
        boot_id: boot_id?.to_owned(),
    })
}

impl RootRunRow {
    pub(crate) fn driver(&self) -> Option<ProcessIdentity> {
        identity(
            self.driver_pid,
            self.driver_start_time,
            self.driver_boot_id.as_deref(),
        )
    }

    pub(crate) fn ordinal(&self) -> u32 {
        number(self.ordinal)
    }

    pub(crate) fn runs(&self) -> u32 {
        number(self.runs)
    }
}

impl Store {
    pub(crate) async fn runs_to_show(
        &self,
        subject: Option<&str>,
        terminal: bool,
    ) -> Result<Vec<RootRunRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT w.id, s.name AS subject, w.ordinal, w.state, w.definition_sha256 AS definition, \
             w.cleaned_at IS NOT NULL AS cleaned, \
             w.driver_pid, w.driver_start_time, w.driver_boot_id, w.driver_since, w.finished_at, \
             (SELECT count(*) FROM workflow_run c WHERE c.subject_id = w.subject_id \
              AND c.parent_step_run_id IS NULL) AS runs \
             FROM subject s JOIN workflow_run w ON w.id = ( \
               SELECT r.id FROM workflow_run r WHERE r.subject_id = s.id AND r.parent_step_run_id IS NULL \
               ORDER BY r.state = 'active' DESC, r.ordinal DESC LIMIT 1) \
             WHERE (?1 IS NULL OR s.name = ?1) AND (?2 OR w.state = 'active') ORDER BY s.name",
        )
        .bind(subject)
        .bind(terminal)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn item_runs(&self, root: &str) -> Result<Vec<ItemRunRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT w.id AS run, w.item_key AS key, w.state AS run_state, b.merged_as_sha AS merged_as, \
             s.step_key, s.kind AS step_kind, s.state AS step_state, s.iteration, e.attempt, e.state AS execution_state, \
             e.started_at FROM workflow_run w \
             LEFT JOIN run_branch b ON b.workflow_run_id = w.id \
             LEFT JOIN step_run s ON s.id = ( \
               SELECT c.id FROM step_run c WHERE c.workflow_run_id = w.id \
               AND c.kind IN ('agent', 'operation') \
               ORDER BY c.state IN ('running', 'blocked') DESC, coalesce(c.iteration, 0) DESC, \
               c.state = 'pending' DESC, CASE WHEN c.state = 'pending' THEN c.rowid ELSE -c.rowid END \
               LIMIT 1) \
             LEFT JOIN execution e ON e.id = ( \
               SELECT x.id FROM execution x WHERE x.step_run_id = s.id ORDER BY x.attempt DESC LIMIT 1) \
             WHERE w.root_run_id = ? AND w.parent_step_run_id IS NOT NULL ORDER BY w.item_key",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?)
    }

    /// The running agent Executions holding a slot, per root run of the
    /// project, with the Driver each run recorded.
    pub(crate) async fn slot_holders(&self) -> Result<Vec<SlotHoldersRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT r.driver_pid, r.driver_start_time, r.driver_boot_id, count(*) AS agents \
             FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             JOIN workflow_run r ON r.id = w.root_run_id \
             WHERE e.kind = 'agent' AND e.state = 'running' AND e.slot IS NOT NULL \
             GROUP BY r.id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn gate_views(&self, root: &str) -> Result<Vec<GateViewRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT s.step_key AS key, s.state, s.outcome, w.definition_sha256 AS definition, \
             r.number AS decided, a.actor, a.decided_at \
             FROM step_run s JOIN workflow_run w ON w.id = s.workflow_run_id \
             LEFT JOIN approval a ON a.gate_step_run_id = s.id \
             LEFT JOIN artifact_revision r ON r.id = a.revision_id \
             WHERE s.workflow_run_id = ?1 AND s.kind = 'approval' AND s.id = ( \
               SELECT g.id FROM step_run g WHERE g.workflow_run_id = ?1 AND g.step_key = s.step_key \
               ORDER BY g.iteration DESC, g.created_at DESC, g.rowid DESC LIMIT 1) \
             ORDER BY s.created_at, s.rowid",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn revision_id(
        &self,
        run: &str,
        artifact: &str,
        revision: Option<u32>,
    ) -> Result<Option<String>, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT r.id FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
             WHERE a.workflow_run_id = ? AND a.name = ? AND (?3 IS NULL OR r.number = ?3) \
             ORDER BY r.number DESC LIMIT 1",
        )
        .bind(run)
        .bind(artifact)
        .bind(revision)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub(crate) async fn artifact_summaries(
        &self,
        run: &str,
    ) -> Result<Vec<ArtifactSummaryRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT a.name, (SELECT count(*) FROM artifact_revision c WHERE c.artifact_id = a.id) AS revisions, \
             r.number, r.origin, \
             EXISTS (SELECT 1 FROM approval p WHERE p.revision_id = r.id AND p.decision = 'approve') AS approved, \
             (SELECT count(*) FROM revision_file f WHERE f.revision_id = r.id) AS files \
             FROM artifact a JOIN artifact_revision r ON r.id = ( \
               SELECT x.id FROM artifact_revision x WHERE x.artifact_id = a.id ORDER BY x.number DESC LIMIT 1) \
             WHERE a.workflow_run_id = ? \
             ORDER BY a.rowid",
        )
        .bind(run)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn item_run(
        &self,
        root: &str,
        key: &str,
    ) -> Result<Option<String>, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT id FROM workflow_run WHERE root_run_id = ? AND item_key = ? \
             ORDER BY created_at, id LIMIT 1",
        )
        .bind(root)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub(crate) async fn last_cleanup(&self, root: &str) -> Result<Option<String>, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT data FROM event WHERE root_run_id = ? \
             AND kind IN ('cleanup.finished', 'cleanup.incomplete') ORDER BY seq DESC LIMIT 1",
        )
        .bind(root)
        .fetch_optional(&self.pool)
        .await?)
    }
}
