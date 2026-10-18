use std::collections::BTreeMap;

use sqlx::{FromRow, SqliteConnection};

use super::revisions::{Origin, number, publish};
use super::{About, Store, StoreError, record_event, unix_now};

const EDIT: &str = "edit";

#[derive(Clone, Debug)]
pub(crate) struct WorkingCopyArtifact {
    pub(crate) name: String,
    pub(crate) id: String,
    pub(crate) run: String,
    pub(crate) files: Vec<BaselineFile>,
    pub(crate) pending: bool,
    pub(crate) pinned: Option<u32>,
    pub(crate) revision: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub(crate) struct BaselineFile {
    pub(crate) rel_path: String,
    pub(crate) sha256: String,
}

pub(crate) struct EditRecord<'a> {
    pub(crate) artifact: &'a WorkingCopyArtifact,
    pub(crate) files: &'a [(String, Vec<u8>)],
    pub(crate) actor: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub(crate) struct PendingFile {
    pub(crate) run: String,
    pub(crate) rel_path: String,
    pub(crate) sha256: String,
    pub(crate) pending_sha256: String,
    pub(crate) pending_revision: String,
}

#[derive(FromRow)]
struct BaselineRow {
    name: String,
    id: String,
    run: String,
    rel_path: String,
    sha256: String,
    pending: bool,
    pinned: Option<i64>,
    revision: i64,
}

impl Store {
    pub(crate) async fn run_definition(&self, run: &str) -> Result<String, StoreError> {
        Ok(
            sqlx::query_scalar("SELECT definition_sha256 FROM workflow_run WHERE id = ?")
                .bind(run)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub(crate) async fn working_copy(
        &self,
        root: &str,
    ) -> Result<Vec<WorkingCopyArtifact>, StoreError> {
        let rows: Vec<BaselineRow> = sqlx::query_as(
            "SELECT a.name, a.id, a.workflow_run_id AS run, w.rel_path, w.sha256, \
             w.pending_revision_id IS NOT NULL AS pending, \
             (SELECT max(r.number) FROM artifact_revision r \
              WHERE r.artifact_id = a.id AND r.pinned_at IS NOT NULL) AS pinned, \
             b.number AS revision FROM working_copy_file w \
             JOIN artifact a ON a.id = w.artifact_id \
             JOIN artifact_revision b ON b.id = w.revision_id \
             WHERE a.workflow_run_id = ? ORDER BY a.name, w.rel_path",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?;
        let mut artifacts: BTreeMap<String, WorkingCopyArtifact> = BTreeMap::new();
        for row in rows {
            let artifact =
                artifacts
                    .entry(row.name.clone())
                    .or_insert_with(|| WorkingCopyArtifact {
                        name: row.name,
                        id: row.id,
                        run: row.run,
                        files: Vec::new(),
                        pending: false,
                        pinned: row.pinned.map(number),
                        revision: number(row.revision),
                    });
            artifact.pending |= row.pending;
            artifact.files.push(BaselineFile {
                rel_path: row.rel_path,
                sha256: row.sha256,
            });
        }
        Ok(artifacts.into_values().collect())
    }

    pub(crate) async fn pending_working_copy(
        &self,
        root: &str,
    ) -> Result<Vec<PendingFile>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT a.workflow_run_id AS run, w.rel_path, w.sha256, w.pending_sha256, \
             w.pending_revision_id AS pending_revision FROM working_copy_file w \
             JOIN artifact a ON a.id = w.artifact_id \
             WHERE a.workflow_run_id = ? AND w.pending_revision_id IS NOT NULL \
             ORDER BY w.pending_revision_id, w.rel_path",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn blob(&self, sha256: &str) -> Result<Vec<u8>, StoreError> {
        Ok(
            sqlx::query_scalar("SELECT bytes FROM blob WHERE sha256 = ?")
                .bind(sha256)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub(crate) async fn record_edit(
        &self,
        edit: EditRecord<'_>,
    ) -> Result<Option<u32>, StoreError> {
        let now = unix_now();
        let artifact = edit.artifact;
        let mut transaction = self.begin_write().await?;
        if baseline(&mut transaction, &artifact.id).await? != artifact.files {
            return Ok(None);
        }
        let origin = Origin::Human {
            action: EDIT,
            actor: edit.actor,
        };
        let published = publish(&mut transaction, &artifact.id, edit.files, origin, now).await?;
        sqlx::query("DELETE FROM working_copy_file WHERE artifact_id = ?")
            .bind(&artifact.id)
            .execute(&mut *transaction)
            .await?;
        for (rel_path, sha256) in &published.files {
            sqlx::query(
                "INSERT INTO working_copy_file (artifact_id, rel_path, sha256, revision_id) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(&artifact.id)
            .bind(rel_path)
            .bind(sha256)
            .bind(&published.revision_id)
            .execute(&mut *transaction)
            .await?;
        }
        record_event(
            &mut transaction,
            About::Run(&artifact.run),
            "working_copy.edit_detected",
            now,
        )
        .await?;
        transaction.commit().await?;
        Ok(Some(published.number))
    }
}

async fn baseline(
    connection: &mut SqliteConnection,
    artifact: &str,
) -> Result<Vec<BaselineFile>, sqlx::Error> {
    sqlx::query_as(
        "SELECT rel_path, sha256 FROM working_copy_file WHERE artifact_id = ? \
         AND pending_revision_id IS NULL ORDER BY rel_path",
    )
    .bind(artifact)
    .fetch_all(connection)
    .await
}
