use sqlx::{FromRow, SqliteConnection};

use super::columns::number;
use super::details::{self, ApprovalDetails};
use super::revisions::{Origin, artifact_id, publish};
use super::runs::new_id;
use super::{About, Store, StoreError, record_event_data, unix_now};

const REJECTION_REASON: &str = "rejection_reason";
const RECORDED: &str = "approval.recorded";
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
        let latest =
            match latest(&mut transaction, &new.gate.run, new.artifact, new.expected).await? {
                Ok(latest) => latest,
                Err(outcome) => return Ok(outcome),
            };
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
        let data = details::decision_recorded(&new, number(latest.number));
        record_event_data(&mut transaction, About::Step(gate), RECORDED, now, &data).await?;
        let outcome = serde_json::json!({ "outcome": new.decision.outcome() });
        record_event_data(
            &mut transaction,
            About::Step(gate),
            "step.done",
            now,
            &outcome,
        )
        .await?;
        transaction.commit().await?;
        Ok(latest.recorded())
    }

    pub(crate) async fn approve_latest(
        &self,
        run: &str,
        artifact: &str,
        expected: Option<u32>,
        actor: &str,
    ) -> Result<DecisionOutcome, StoreError> {
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        let latest = match latest(&mut transaction, run, artifact, expected).await? {
            Ok(latest) => latest,
            Err(outcome) => return Ok(outcome),
        };
        let inserted = sqlx::query(
            "INSERT INTO approval (id, revision_id, identity_sha256, decision, actor, decided_at) \
             VALUES (?, ?, ?, 'approve', ?, ?) ON CONFLICT (revision_id) DO NOTHING",
        )
        .bind(new_id())
        .bind(&latest.id)
        .bind(&latest.identity_sha256)
        .bind(actor)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        if inserted.rows_affected() == 0 {
            return Ok(DecisionOutcome::NoRevision);
        }
        let data = details::approval_recorded(&ApprovalDetails {
            decision: "approve",
            artifact,
            revision: number(latest.number),
            actor,
            reason: None,
        });
        record_event_data(&mut transaction, About::Run(run), RECORDED, now, &data).await?;
        transaction.commit().await?;
        Ok(latest.recorded())
    }

    pub(crate) async fn unapproved_after_approval(
        &self,
        root: &str,
    ) -> Result<Vec<(String, u32)>, StoreError> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT a.name, r.number FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
             WHERE a.workflow_run_id = ? \
             AND r.number = (SELECT max(number) FROM artifact_revision WHERE artifact_id = a.id) \
             AND NOT EXISTS (SELECT 1 FROM approval p WHERE p.revision_id = r.id) \
             AND EXISTS (SELECT 1 FROM approval p JOIN artifact_revision o ON o.id = p.revision_id \
                         WHERE o.artifact_id = a.id AND p.decision = 'approve') \
             ORDER BY a.name",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, revision)| (name, number(revision)))
            .collect())
    }
}

impl Latest {
    fn recorded(self) -> DecisionOutcome {
        DecisionOutcome::Recorded {
            number: number(self.number),
            from_execution: self.origin == "execution",
            revision_id: self.id,
            identity: self.identity_sha256,
        }
    }
}

async fn latest(
    connection: &mut SqliteConnection,
    run: &str,
    artifact: &str,
    expected: Option<u32>,
) -> Result<Result<Latest, DecisionOutcome>, sqlx::Error> {
    let latest: Option<Latest> = sqlx::query_as(
        "SELECT r.id, r.number, r.identity_sha256, r.origin FROM artifact a \
         JOIN artifact_revision r ON r.artifact_id = a.id \
         WHERE a.workflow_run_id = ? AND a.name = ? ORDER BY r.number DESC LIMIT 1",
    )
    .bind(run)
    .bind(artifact)
    .fetch_optional(connection)
    .await?;
    let Some(latest) = latest else {
        return Ok(Err(DecisionOutcome::NoRevision));
    };
    let latest_number = number(latest.number);
    Ok(
        match expected.filter(|expected| *expected != latest_number) {
            Some(expected) => Err(DecisionOutcome::Stale {
                expected,
                latest: latest_number,
            }),
            None => Ok(latest),
        },
    )
}
