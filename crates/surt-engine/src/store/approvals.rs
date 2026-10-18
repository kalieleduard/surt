use sqlx::FromRow;

use super::revisions::{Origin, artifact_id, number, publish};
use super::runs::new_id;
use super::{About, Store, StoreError, record_event, unix_now};

const REJECTION_REASON: &str = "rejection_reason";
const REASON_EXTENSION: &str = "txt";

#[derive(Clone, Debug, FromRow)]
pub(crate) struct WaitingGate {
    pub(crate) id: String,
    pub(crate) run: String,
    pub(crate) key: String,
    pub(crate) definition: String,
}

pub(crate) struct NewDecision<'a> {
    pub(crate) gate: &'a WaitingGate,
    pub(crate) artifact: &'a str,
    pub(crate) expected: Option<u32>,
    pub(crate) decision: Decision<'a>,
    pub(crate) actor: &'a str,
}

#[derive(Clone, Copy)]
pub(crate) enum Decision<'a> {
    Approve,
    Reject {
        rejection_artifact: &'a str,
        reason: &'a str,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DecisionOutcome {
    Recorded {
        revision_id: String,
        number: u32,
        identity: String,
        from_execution: bool,
    },
    GateNotWaiting,
    NoRevision,
    Stale {
        expected: u32,
        latest: u32,
    },
}

#[derive(FromRow)]
struct Latest {
    id: String,
    number: i64,
    identity_sha256: String,
    origin: String,
}

impl Decision<'_> {
    fn name(self) -> &'static str {
        match self {
            Decision::Approve => "approve",
            Decision::Reject { .. } => "reject",
        }
    }

    fn outcome(self) -> &'static str {
        match self {
            Decision::Approve => "approved",
            Decision::Reject { .. } => "rejected",
        }
    }
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct GateRow {
    pub(crate) state: String,
    pub(crate) outcome: Option<String>,
    pub(crate) definition: String,
    decided: Option<i64>,
}

impl GateRow {
    pub(crate) fn decided(&self) -> Option<u32> {
        self.decided.map(number)
    }
}

impl Store {
    pub(crate) async fn latest_gate(
        &self,
        root: &str,
        key: &str,
    ) -> Result<Option<GateRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT s.state, s.outcome, w.definition_sha256 AS definition, r.number AS decided \
             FROM step_run s JOIN workflow_run w ON w.id = s.workflow_run_id \
             LEFT JOIN approval a ON a.gate_step_run_id = s.id \
             LEFT JOIN artifact_revision r ON r.id = a.revision_id \
             WHERE s.workflow_run_id = ? AND s.step_key = ? AND s.kind = 'approval' \
             ORDER BY s.iteration DESC, s.created_at DESC LIMIT 1",
        )
        .bind(root)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub(crate) async fn latest_revision_number(
        &self,
        run: &str,
        artifact: &str,
    ) -> Result<Option<u32>, StoreError> {
        let latest: Option<i64> = sqlx::query_scalar(
            "SELECT max(r.number) FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
             WHERE a.workflow_run_id = ? AND a.name = ?",
        )
        .bind(run)
        .bind(artifact)
        .fetch_one(&self.pool)
        .await?;
        Ok(latest.map(number))
    }

    pub(crate) async fn waiting_gates(&self, root: &str) -> Result<Vec<WaitingGate>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT s.id, s.workflow_run_id AS run, s.step_key AS key, w.definition_sha256 AS definition \
             FROM step_run s JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.root_run_id = ? AND s.kind = 'approval' AND s.state = 'running' \
             ORDER BY s.created_at, s.id",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn record_decision(
        &self,
        new: NewDecision<'_>,
    ) -> Result<DecisionOutcome, StoreError> {
        let now = unix_now();
        let gate = &new.gate.id;
        let mut transaction = self.begin_write().await?;
        let latest: Option<Latest> = sqlx::query_as(
            "SELECT r.id, r.number, r.identity_sha256, r.origin FROM artifact a \
             JOIN artifact_revision r ON r.artifact_id = a.id \
             WHERE a.workflow_run_id = ? AND a.name = ? ORDER BY r.number DESC LIMIT 1",
        )
        .bind(&new.gate.run)
        .bind(new.artifact)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(latest) = latest else {
            return Ok(DecisionOutcome::NoRevision);
        };
        let latest_number = number(latest.number);
        if let Some(expected) = new.expected.filter(|expected| *expected != latest_number) {
            return Ok(DecisionOutcome::Stale {
                expected,
                latest: latest_number,
            });
        }
        let closed = sqlx::query(
            "UPDATE step_run SET state = 'done', outcome = ?, finished_at = ? WHERE id = ? AND state = 'running'",
        )
        .bind(new.decision.outcome())
        .bind(now)
        .bind(gate)
        .execute(&mut *transaction)
        .await?;
        if closed.rows_affected() == 0 {
            return Ok(DecisionOutcome::GateNotWaiting);
        }
        let reason = match new.decision {
            Decision::Approve => None,
            Decision::Reject {
                rejection_artifact,
                reason,
            } => {
                let artifact =
                    artifact_id(&mut transaction, &new.gate.run, rejection_artifact).await?;
                let files = [(
                    format!("{rejection_artifact}.{REASON_EXTENSION}"),
                    reason.as_bytes().to_vec(),
                )];
                let origin = Origin::Human {
                    action: REJECTION_REASON,
                    actor: new.actor,
                };
                Some(
                    publish(&mut transaction, &artifact, &files, origin, now)
                        .await?
                        .revision_id,
                )
            }
        };
        sqlx::query(
            "INSERT INTO approval (id, revision_id, identity_sha256, gate_step_run_id, decision, reason_revision_id, actor, decided_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(new_id())
        .bind(&latest.id)
        .bind(&latest.identity_sha256)
        .bind(gate)
        .bind(new.decision.name())
        .bind(reason)
        .bind(new.actor)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        record_event(
            &mut transaction,
            About::Step(gate),
            "approval.recorded",
            now,
        )
        .await?;
        record_event(&mut transaction, About::Step(gate), "step.done", now).await?;
        transaction.commit().await?;
        Ok(DecisionOutcome::Recorded {
            revision_id: latest.id,
            number: latest_number,
            identity: latest.identity_sha256,
            from_execution: latest.origin == "execution",
        })
    }
}
