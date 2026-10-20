use sqlx::SqlitePool;

const ROWS_IN_EVERY_TABLE: [&str; 29] = [
    "INSERT INTO project (project_id, created_at) VALUES ('my-app-0a1b2c', 1792224000)",
    "INSERT INTO blob (sha256, size, bytes) VALUES ('dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd', 5, x'68656c6c6f')",
    "INSERT INTO snapshot (sha256, kind, name) VALUES ('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'workflow', 'feature')",
    "INSERT INTO snapshot (sha256, kind, name) VALUES ('bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', 'agent', 'spec')",
    "INSERT INTO snapshot (sha256, kind, name) VALUES ('cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc', 'config', 'config')",
    "INSERT INTO snapshot_file (snapshot_sha256, rel_path, blob_sha256) VALUES ('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'feature.toml', 'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd')",
    "INSERT INTO subject (id, name, created_at) VALUES ('subject-1', '20261017-add-login', 1792224000)",
    "INSERT INTO workflow_run (id, subject_id, root_run_id, definition_sha256, ordinal, config_sha256, state, created_at, driver_pid, driver_start_time, driver_boot_id, driver_since) VALUES ('run-1', 'subject-1', 'run-1', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 1, 'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc', 'active', 1792224000, 4242, 1792223990, 'boot-1', 1792224000)",
    "INSERT INTO run_snapshot (root_run_id, snapshot_sha256) VALUES ('run-1', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')",
    "INSERT INTO step_run (id, workflow_run_id, step_key, kind, state, budget_base, created_at, started_at) VALUES ('step-loop', 'run-1', 'spec-loop', 'loop', 'running', 1, 1792224000, 1792224000)",
    "INSERT INTO step_run (id, workflow_run_id, step_key, kind, loop_step_run_id, iteration, state, outcome, created_at, started_at, finished_at) VALUES ('step-spec', 'run-1', 'spec', 'agent', 'step-loop', 1, 'done', 'done', 1792224000, 1792224010, 1792224100)",
    "INSERT INTO step_run (id, workflow_run_id, step_key, kind, loop_step_run_id, iteration, state, created_at) VALUES ('step-gate', 'run-1', 'approve-spec', 'approval', 'step-loop', 1, 'pending', 1792224100)",
    "INSERT INTO step_run (id, workflow_run_id, step_key, kind, state, blocked_reason, created_at, started_at) VALUES ('step-prepare', 'run-1', 'prepare', 'operation', 'blocked', 'not_retryable', 1792224000, 1792224000)",
    "INSERT INTO step_run (id, workflow_run_id, step_key, kind, state, created_at, started_at) VALUES ('step-items', 'run-1', 'build', 'per_item', 'running', 1792224100, 1792224100)",
    "INSERT INTO workflow_run (id, subject_id, root_run_id, parent_step_run_id, item_key, definition_sha256, state, created_at) VALUES ('run-1-a', 'subject-1', 'run-1', 'step-items', 'a', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'active', 1792224100)",
    "INSERT INTO execution (id, step_run_id, attempt, kind, agent_snapshot_sha256, state, outcome, slot, runtime_info, queued_at, started_at, finished_at) VALUES ('execution-spec', 'step-spec', 1, 'agent', 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', 'succeeded', 'done', 0, '{\"session_id\":\"session-1\"}', 1792224005, 1792224010, 1792224100)",
    "INSERT INTO execution (id, step_run_id, attempt, kind, operation_id, state, failure_class, error_code, error_message, started_at, finished_at) VALUES ('execution-prepare', 'step-prepare', 1, 'operation', 'git.prepare', 'failed', 'not_retryable', 'dirty', 'the worktree is dirty', 1792224000, 1792224001)",
    "INSERT INTO artifact (id, workflow_run_id, name) VALUES ('artifact-spec', 'run-1', 'spec')",
    "INSERT INTO artifact (id, workflow_run_id, name) VALUES ('artifact-reason', 'run-1', 'spec-rejection')",
    "INSERT INTO artifact_revision (id, artifact_id, number, identity_sha256, origin, execution_id, pinned_at, created_at) VALUES ('revision-spec', 'artifact-spec', 1, 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee', 'execution', 'execution-spec', 1792224150, 1792224100)",
    "INSERT INTO artifact_revision (id, artifact_id, number, identity_sha256, origin, human_action, actor, created_at) VALUES ('revision-edit', 'artifact-spec', 2, 'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff', 'human', 'edit', 'dev@example.com', 1792224200)",
    "INSERT INTO artifact_revision (id, artifact_id, number, identity_sha256, origin, human_action, actor, created_at) VALUES ('revision-reason', 'artifact-reason', 1, '1111111111111111111111111111111111111111111111111111111111111111', 'human', 'rejection_reason', 'dev@example.com', 1792224200)",
    "INSERT INTO revision_file (revision_id, rel_path, blob_sha256) VALUES ('revision-spec', 'spec.md', 'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd')",
    "INSERT INTO approval (id, revision_id, identity_sha256, gate_step_run_id, decision, reason_revision_id, actor, decided_at) VALUES ('approval-1', 'revision-spec', 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee', 'step-gate', 'reject', 'revision-reason', 'dev@example.com', 1792224200)",
    "INSERT INTO working_copy_file (artifact_id, rel_path, sha256, revision_id, pending_sha256, pending_revision_id) VALUES ('artifact-spec', 'spec.md', 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee', 'revision-spec', 'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff', 'revision-edit')",
    "INSERT INTO execution_input (execution_id, input_name, revision_id) VALUES ('execution-spec', 'rejection', 'revision-reason')",
    "INSERT INTO run_branch (workflow_run_id, ref_name, start_sha, tip_sha, worktree_path, created_at) VALUES ('run-1', 'surt/20261017-add-login', '2222222222222222222222222222222222222222', '3333333333333333333333333333333333333333', '.surt/worktrees/run-1', 1792224000)",
    "INSERT INTO ref_move (step_run_id, ref_name, old_sha, new_sha, created_at) VALUES ('step-prepare', 'refs/heads/surt/20261017-add-login', '2222222222222222222222222222222222222222', '3333333333333333333333333333333333333333', 1792224001)",
    "INSERT INTO event (at, root_run_id, workflow_run_id, step_run_id, execution_id, kind, data) VALUES (1792224100, 'run-1', 'run-1', 'step-spec', 'execution-spec', 'execution.succeeded', '{\"outcome\":\"done\"}')",
];

pub(super) async fn fill(pool: &SqlitePool) {
    let mut transaction = pool.begin().await.expect("a transaction starts");
    for statement in ROWS_IN_EVERY_TABLE {
        sqlx::query(statement)
            .execute(&mut *transaction)
            .await
            .unwrap_or_else(|error| panic!("{statement}: {error}"));
    }
    transaction.commit().await.expect("the fixture commits");
}
