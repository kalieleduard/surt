use sqlx::SqliteConnection;

use super::runs::new_id;
use super::{About, Checkpoint, Store, StoreError, record_event, unix_now};

const RUN_CREATED: &str = "run.created";

pub(crate) struct Expansion<'a> {
    pub(crate) step: &'a str,
    pub(crate) revision: &'a str,
    pub(crate) children: Vec<NewChild<'a>>,
}

pub(crate) struct NewChild<'a> {
    pub(crate) item: &'a str,
    pub(crate) definition: &'a str,
    pub(crate) steps: Vec<(&'a str, &'a str)>,
}

pub(crate) struct ListRevision {
    pub(crate) id: String,
    pub(crate) files: Vec<(String, Vec<u8>)>,
}

impl Store {
    pub(crate) async fn latest_revision(
        &self,
        run: &str,
        artifact: &str,
    ) -> Result<Option<ListRevision>, StoreError> {
        self.list_revision(
            "SELECT r.id FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
             WHERE a.workflow_run_id = ? AND a.name = ? ORDER BY r.number DESC LIMIT 1",
            run,
            artifact,
        )
        .await
    }

    pub(crate) async fn expanded_revision(
        &self,
        run: &str,
        artifact: &str,
    ) -> Result<Option<ListRevision>, StoreError> {
        self.list_revision(
            "SELECT r.id FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
             WHERE a.workflow_run_id = ? AND a.name = ? AND r.pinned_at IS NOT NULL \
             ORDER BY r.number DESC LIMIT 1",
            run,
            artifact,
        )
        .await
    }

    pub(crate) async fn expand_items(&self, expansion: Expansion<'_>) -> Result<(), StoreError> {
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        let opened = sqlx::query(
            "UPDATE step_run SET state = 'running', started_at = coalesce(started_at, ?) \
             WHERE id = ? AND state = 'pending'",
        )
        .bind(now)
        .bind(expansion.step)
        .execute(&mut *transaction)
        .await?;
        if opened.rows_affected() == 0 {
            return Ok(());
        }
        record_event(
            &mut transaction,
            About::Step(expansion.step),
            "step.running",
            now,
        )
        .await?;
        for child in &expansion.children {
            insert_child(&mut transaction, expansion.step, child, now).await?;
        }
        let pinned = sqlx::query(
            "UPDATE artifact_revision SET pinned_at = ? WHERE id = ? AND pinned_at IS NULL",
        )
        .bind(now)
        .bind(expansion.revision)
        .execute(&mut *transaction)
        .await?;
        if pinned.rows_affected() > 0 {
            let about = About::Step(expansion.step);
            record_event(&mut transaction, about, "revision.pinned", now).await?;
        }
        transaction.commit().await?;
        self.reached(Checkpoint::IterationOrExpansion)
    }

    async fn list_revision(
        &self,
        sql: &'static str,
        run: &str,
        artifact: &str,
    ) -> Result<Option<ListRevision>, StoreError> {
        let id: Option<String> = sqlx::query_scalar(sql)
            .bind(run)
            .bind(artifact)
            .fetch_optional(&self.pool)
            .await?;
        let Some(id) = id else {
            return Ok(None);
        };
        let files = sqlx::query_as(
            "SELECT f.rel_path, b.bytes FROM revision_file f \
             JOIN blob b ON b.sha256 = f.blob_sha256 WHERE f.revision_id = ? ORDER BY f.rel_path",
        )
        .bind(&id)
        .fetch_all(&self.pool)
        .await?;
        Ok(Some(ListRevision { id, files }))
    }
}

async fn insert_child(
    connection: &mut SqliteConnection,
    step: &str,
    child: &NewChild<'_>,
    now: i64,
) -> Result<(), sqlx::Error> {
    let run = new_id();
    sqlx::query(
        "INSERT INTO workflow_run (id, subject_id, root_run_id, parent_step_run_id, item_key, \
         definition_sha256, state, created_at) \
         SELECT ?, w.subject_id, w.root_run_id, s.id, ?, ?, 'active', ? FROM step_run s \
         JOIN workflow_run w ON w.id = s.workflow_run_id WHERE s.id = ?",
    )
    .bind(&run)
    .bind(child.item)
    .bind(child.definition)
    .bind(now)
    .bind(step)
    .execute(&mut *connection)
    .await?;
    for (key, kind) in &child.steps {
        sqlx::query(
            "INSERT INTO step_run (id, workflow_run_id, step_key, kind, state, created_at) \
             VALUES (?, ?, ?, ?, 'pending', ?)",
        )
        .bind(new_id())
        .bind(&run)
        .bind(key)
        .bind(kind)
        .bind(now)
        .execute(&mut *connection)
        .await?;
    }
    record_event(connection, About::Run(&run), RUN_CREATED, now).await
}
