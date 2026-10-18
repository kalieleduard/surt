use sqlx::FromRow;

use super::{Store, StoreError};

#[derive(Clone, Debug, FromRow)]
pub(crate) struct FrozenFile {
    pub(crate) snapshot: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) rel_path: String,
    pub(crate) bytes: Vec<u8>,
}

impl Store {
    pub(crate) async fn frozen_files(&self, root: &str) -> Result<Vec<FrozenFile>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT s.sha256 AS snapshot, s.kind, s.name, f.rel_path, b.bytes FROM run_snapshot r \
             JOIN snapshot s ON s.sha256 = r.snapshot_sha256 \
             JOIN snapshot_file f ON f.snapshot_sha256 = s.sha256 \
             JOIN blob b ON b.sha256 = f.blob_sha256 \
             WHERE r.root_run_id = ? ORDER BY s.sha256, f.rel_path",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?)
    }
}
