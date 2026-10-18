use serde_json::json;
use sqlx::SqliteConnection;

use super::revisions::{Origin, Published, artifact_id, publish};
use super::{About, Checkpoint, Store, StoreError, record_event, record_event_data, unix_now};

pub(crate) struct Acceptance<'a> {
    pub(crate) execution: &'a str,
    pub(crate) outcome: &'a str,
    pub(crate) outputs: &'a [PublishedOutput],
    pub(crate) tip: Option<&'a str>,
    pub(crate) integrated: Option<Integrated<'a>>,
}

#[derive(Clone, Copy)]
pub(crate) struct Integrated<'a> {
    pub(crate) into: &'a str,
    pub(crate) merge: &'a str,
}

pub(crate) struct PublishedOutput {
    pub(crate) name: String,
    pub(crate) files: Vec<(String, Vec<u8>)>,
    pub(crate) working_copy: bool,
}

impl Store {
    pub(crate) async fn accept_execution(
        &self,
        acceptance: Acceptance<'_>,
    ) -> Result<Vec<(String, Published)>, StoreError> {
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        let (step, run): (String, String) = sqlx::query_as(
            "SELECT s.id, s.workflow_run_id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             WHERE e.id = ?",
        )
        .bind(acceptance.execution)
        .fetch_one(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE execution SET state = 'succeeded', outcome = ?, finished_at = ? WHERE id = ?",
        )
        .bind(acceptance.outcome)
        .bind(now)
        .bind(acceptance.execution)
        .execute(&mut *transaction)
        .await?;
        let mut published = Vec::new();
        for output in acceptance.outputs {
            let artifact = artifact_id(&mut transaction, &run, &output.name).await?;
            let origin = Origin::Execution(acceptance.execution);
            let revision = publish(&mut transaction, &artifact, &output.files, origin, now).await?;
            if output.working_copy {
                stage_working_copy(&mut transaction, &revision).await?;
            }
            let about = About::Execution(acceptance.execution);
            let data = json!({ "artifact": output.name, "revision": revision.number, "origin": "execution" });
            record_event_data(&mut transaction, about, "revision.published", now, &data).await?;
            published.push((output.name.clone(), revision));
        }
        if let Some(tip) = acceptance.tip {
            sqlx::query("UPDATE run_branch SET tip_sha = ? WHERE workflow_run_id = ?")
                .bind(tip)
                .bind(&run)
                .execute(&mut *transaction)
                .await?;
        }
        if let Some(integrated) = acceptance.integrated {
            record_integration(&mut transaction, &run, integrated).await?;
        }
        sqlx::query("DELETE FROM ref_move WHERE step_run_id = ?")
            .bind(&step)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "UPDATE step_run SET state = 'done', outcome = ?, finished_at = ?, blocked_reason = NULL WHERE id = ?",
        )
        .bind(acceptance.outcome)
        .bind(now)
        .bind(&step)
        .execute(&mut *transaction)
        .await?;
        let about = About::Execution(acceptance.execution);
        record_event(&mut transaction, about, "execution.succeeded", now).await?;
        record_event(&mut transaction, About::Step(&step), "step.done", now).await?;
        transaction.commit().await?;
        self.reached(Checkpoint::ExecutionAccepted)?;
        Ok(published)
    }

    pub(crate) async fn promote_working_copy(
        &self,
        run: &str,
        revision: &str,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "UPDATE working_copy_file SET sha256 = pending_sha256, revision_id = pending_revision_id, \
             pending_sha256 = NULL, pending_revision_id = NULL WHERE pending_revision_id = ?",
        )
        .bind(revision)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "DELETE FROM working_copy_file WHERE revision_id <> ?1 AND pending_revision_id IS NULL \
             AND artifact_id = (SELECT artifact_id FROM artifact_revision WHERE id = ?1)",
        )
        .bind(revision)
        .execute(&mut *transaction)
        .await?;
        record_event(
            &mut transaction,
            About::Run(run),
            "working_copy.written",
            unix_now(),
        )
        .await?;
        transaction.commit().await?;
        self.reached(Checkpoint::WorkingCopyWritten)
    }

    pub(crate) async fn superseded_working_copy_files(
        &self,
        revision: &str,
    ) -> Result<Vec<String>, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT w.rel_path FROM working_copy_file w \
             JOIN artifact_revision r ON r.artifact_id = w.artifact_id \
             WHERE r.id = ?1 AND w.pending_revision_id IS NULL \
             AND w.rel_path NOT IN (SELECT rel_path FROM revision_file WHERE revision_id = ?1) \
             ORDER BY w.rel_path",
        )
        .bind(revision)
        .fetch_all(&self.pool)
        .await?)
    }
}

async fn stage_working_copy(
    connection: &mut SqliteConnection,
    revision: &Published,
) -> Result<(), sqlx::Error> {
    for (rel_path, sha256) in &revision.files {
        sqlx::query(
            "INSERT INTO working_copy_file (artifact_id, rel_path, sha256, revision_id, pending_sha256, pending_revision_id) \
             VALUES (?1, ?2, ?3, ?4, ?3, ?4) ON CONFLICT (artifact_id, rel_path) DO UPDATE \
             SET pending_sha256 = excluded.pending_sha256, pending_revision_id = excluded.pending_revision_id",
        )
        .bind(&revision.artifact_id)
        .bind(rel_path)
        .bind(sha256)
        .bind(&revision.revision_id)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

async fn record_integration(
    connection: &mut SqliteConnection,
    run: &str,
    integrated: Integrated<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE run_branch SET tip_sha = ? WHERE workflow_run_id = ?")
        .bind(integrated.merge)
        .bind(integrated.into)
        .execute(&mut *connection)
        .await?;
    sqlx::query("UPDATE run_branch SET merged_as_sha = ? WHERE workflow_run_id = ?")
        .bind(integrated.merge)
        .bind(run)
        .execute(&mut *connection)
        .await?;
    Ok(())
}
