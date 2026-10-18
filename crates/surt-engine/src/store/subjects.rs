use super::Store;

impl Store {
    pub(crate) async fn names_of_subjects_with_active_runs(
        &self,
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT subject.name FROM subject JOIN workflow_run ON workflow_run.subject_id = subject.id \
             WHERE workflow_run.parent_step_run_id IS NULL AND workflow_run.state = 'active' \
             ORDER BY subject.name DESC",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub(crate) async fn subject_names_starting_with(
        &self,
        stem: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT name FROM subject WHERE substr(name, 1, length(?1)) = ?1")
            .bind(stem)
            .fetch_all(&self.pool)
            .await
    }

    pub(crate) async fn subject_of_run(&self, run: &str) -> Result<String, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT s.name FROM subject s JOIN workflow_run w ON w.subject_id = s.id WHERE w.id = ?",
        )
        .bind(run)
        .fetch_one(&self.pool)
        .await
    }
}
