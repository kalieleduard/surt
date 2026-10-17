CREATE TABLE project (
  project_id TEXT PRIMARY KEY,
  created_at INTEGER NOT NULL
) STRICT;

CREATE TRIGGER project_single_row BEFORE INSERT ON project
WHEN (SELECT count(*) FROM project) > 0
BEGIN
  SELECT RAISE(ABORT, 'project has exactly one row');
END;

CREATE TABLE blob (
  sha256 TEXT PRIMARY KEY CHECK (length(sha256) = 64),
  size   INTEGER NOT NULL CHECK (size = length(bytes)),
  bytes  BLOB NOT NULL
) STRICT;

CREATE TABLE snapshot (
  sha256 TEXT PRIMARY KEY CHECK (length(sha256) = 64),
  kind   TEXT NOT NULL CHECK (kind IN ('workflow', 'agent', 'config')),
  name   TEXT NOT NULL
) STRICT;

CREATE TABLE snapshot_file (
  snapshot_sha256 TEXT NOT NULL REFERENCES snapshot(sha256) ON DELETE CASCADE,
  rel_path        TEXT NOT NULL,
  blob_sha256     TEXT NOT NULL REFERENCES blob(sha256),
  PRIMARY KEY (snapshot_sha256, rel_path)
) STRICT;
CREATE INDEX snapshot_file_by_blob ON snapshot_file(blob_sha256);

CREATE TABLE subject (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL UNIQUE COLLATE NOCASE,
  created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE workflow_run (
  id                  TEXT PRIMARY KEY,
  subject_id          TEXT NOT NULL REFERENCES subject(id) ON DELETE CASCADE,
  root_run_id         TEXT NOT NULL REFERENCES workflow_run(id) ON DELETE CASCADE,
  parent_step_run_id  TEXT REFERENCES step_run(id) ON DELETE CASCADE,
  item_key            TEXT,
  definition_sha256   TEXT NOT NULL REFERENCES snapshot(sha256),
  ordinal             INTEGER CHECK (ordinal >= 1),
  config_sha256       TEXT REFERENCES snapshot(sha256),
  state               TEXT NOT NULL CHECK (state IN ('active', 'succeeded', 'cancelled')),
  created_at          INTEGER NOT NULL,
  finished_at         INTEGER,
  cancel_requested_at INTEGER,
  cancel_requested_by TEXT,
  stop_requested_at   INTEGER,
  stop_requested_by   TEXT,
  driver_pid          INTEGER,
  driver_start_time   INTEGER,
  driver_boot_id      TEXT,
  driver_since        INTEGER,
  cleaned_at          INTEGER,
  CHECK ((id = root_run_id) = (parent_step_run_id IS NULL)),
  CHECK ((parent_step_run_id IS NULL) = (item_key IS NULL)),
  CHECK ((parent_step_run_id IS NULL) = (ordinal IS NOT NULL)),
  CHECK ((parent_step_run_id IS NULL) = (config_sha256 IS NOT NULL)),
  CHECK (parent_step_run_id IS NULL OR (cancel_requested_at IS NULL AND stop_requested_at IS NULL AND driver_pid IS NULL)),
  CHECK ((cancel_requested_at IS NULL) = (cancel_requested_by IS NULL)),
  CHECK ((stop_requested_at IS NULL) = (stop_requested_by IS NULL)),
  CHECK ((state = 'active') = (finished_at IS NULL)),
  CHECK (cleaned_at IS NULL OR (parent_step_run_id IS NULL AND state <> 'active'))
) STRICT;

CREATE UNIQUE INDEX one_active_root_per_subject ON workflow_run(subject_id)
  WHERE state = 'active' AND parent_step_run_id IS NULL;
CREATE UNIQUE INDEX root_ordinal ON workflow_run(subject_id, ordinal)
  WHERE ordinal IS NOT NULL;
CREATE UNIQUE INDEX one_child_per_item ON workflow_run(parent_step_run_id, item_key)
  WHERE parent_step_run_id IS NOT NULL;
CREATE INDEX workflow_run_by_subject ON workflow_run(subject_id);
CREATE INDEX workflow_run_by_root ON workflow_run(root_run_id, state);
CREATE INDEX workflow_run_by_parent ON workflow_run(parent_step_run_id);

CREATE TABLE run_snapshot (
  root_run_id     TEXT NOT NULL REFERENCES workflow_run(id) ON DELETE CASCADE,
  snapshot_sha256 TEXT NOT NULL REFERENCES snapshot(sha256),
  PRIMARY KEY (root_run_id, snapshot_sha256)
) STRICT;
CREATE INDEX run_snapshot_by_snapshot ON run_snapshot(snapshot_sha256);

CREATE TABLE step_run (
  id               TEXT PRIMARY KEY,
  workflow_run_id  TEXT NOT NULL REFERENCES workflow_run(id) ON DELETE CASCADE,
  step_key         TEXT NOT NULL,
  kind             TEXT NOT NULL CHECK (kind IN ('agent', 'approval', 'operation', 'loop', 'per_item')),
  loop_step_run_id TEXT REFERENCES step_run(id) ON DELETE CASCADE,
  iteration        INTEGER CHECK (iteration >= 1),
  state            TEXT NOT NULL CHECK (state IN ('pending', 'running', 'done', 'skipped', 'blocked', 'cancelled')),
  outcome          TEXT,
  blocked_reason   TEXT CHECK (blocked_reason IN ('attempts_exhausted', 'not_retryable', 'iterations_exhausted')),
  budget_base      INTEGER NOT NULL DEFAULT 0 CHECK (budget_base >= 0),
  created_at       INTEGER NOT NULL,
  started_at       INTEGER,
  finished_at      INTEGER,
  CHECK ((loop_step_run_id IS NULL) = (iteration IS NULL)),
  CHECK (kind <> 'loop' OR loop_step_run_id IS NULL),
  CHECK ((state = 'done') = (outcome IS NOT NULL)),
  CHECK ((state = 'blocked') = (blocked_reason IS NOT NULL)),
  CHECK (state <> 'blocked' OR kind IN ('agent', 'operation', 'loop')),
  CHECK (blocked_reason <> 'iterations_exhausted' OR kind = 'loop'),
  CHECK (state = 'pending' OR state = 'skipped' OR state = 'cancelled' OR started_at IS NOT NULL),
  CHECK ((state IN ('done', 'skipped', 'cancelled')) = (finished_at IS NOT NULL))
) STRICT;

CREATE UNIQUE INDEX one_step_run_outside_loops ON step_run(workflow_run_id, step_key)
  WHERE loop_step_run_id IS NULL;
CREATE UNIQUE INDEX one_step_run_per_iteration ON step_run(loop_step_run_id, iteration, step_key)
  WHERE loop_step_run_id IS NOT NULL;
CREATE INDEX step_run_by_run ON step_run(workflow_run_id, state);
CREATE INDEX step_run_by_loop ON step_run(loop_step_run_id);

CREATE TABLE execution (
  id                    TEXT PRIMARY KEY,
  step_run_id           TEXT NOT NULL REFERENCES step_run(id) ON DELETE CASCADE,
  attempt               INTEGER NOT NULL CHECK (attempt >= 1),
  kind                  TEXT NOT NULL CHECK (kind IN ('agent', 'operation')),
  agent_snapshot_sha256 TEXT REFERENCES snapshot(sha256),
  operation_id          TEXT,
  state                 TEXT NOT NULL CHECK (state IN ('queued', 'running', 'succeeded', 'failed', 'interrupted', 'cancelled')),
  outcome               TEXT,
  failure_class         TEXT CHECK (failure_class IN ('retryable', 'not_retryable', 'transient')),
  error_code            TEXT,
  error_message         TEXT,
  slot                  INTEGER CHECK (slot >= 0),
  pid                   INTEGER,
  process_start_time    INTEGER,
  process_boot_id       TEXT,
  runtime_info          TEXT CHECK (runtime_info IS NULL OR json_valid(runtime_info)),
  queued_at             INTEGER,
  started_at            INTEGER,
  finished_at           INTEGER,
  UNIQUE (step_run_id, attempt),
  CHECK ((kind = 'agent') = (agent_snapshot_sha256 IS NOT NULL)),
  CHECK ((kind = 'operation') = (operation_id IS NOT NULL)),
  CHECK (kind = 'agent' OR (slot IS NULL AND queued_at IS NULL AND state <> 'queued')),
  CHECK ((state = 'succeeded') = (outcome IS NOT NULL)),
  CHECK ((state = 'failed') = (failure_class IS NOT NULL)),
  CHECK (state NOT IN ('running', 'succeeded', 'failed') OR started_at IS NOT NULL),
  CHECK ((state IN ('queued', 'running')) = (finished_at IS NULL))
) STRICT;

CREATE UNIQUE INDEX one_live_execution_per_step_run ON execution(step_run_id)
  WHERE state IN ('queued', 'running');
CREATE INDEX live_execution ON execution(state)
  WHERE state IN ('queued', 'running');

CREATE TABLE artifact (
  id              TEXT PRIMARY KEY,
  workflow_run_id TEXT NOT NULL REFERENCES workflow_run(id) ON DELETE CASCADE,
  name            TEXT NOT NULL,
  UNIQUE (workflow_run_id, name)
) STRICT;

CREATE TABLE artifact_revision (
  id              TEXT PRIMARY KEY,
  artifact_id     TEXT NOT NULL REFERENCES artifact(id) ON DELETE CASCADE,
  number          INTEGER NOT NULL CHECK (number >= 1),
  identity_sha256 TEXT NOT NULL CHECK (length(identity_sha256) = 64),
  origin          TEXT NOT NULL CHECK (origin IN ('execution', 'human')),
  execution_id    TEXT REFERENCES execution(id) ON DELETE CASCADE,
  human_action    TEXT CHECK (human_action IN ('initial', 'edit', 'rejection_reason')),
  actor           TEXT,
  pinned_at       INTEGER,
  created_at      INTEGER NOT NULL,
  UNIQUE (artifact_id, number),
  UNIQUE (execution_id, artifact_id),
  CHECK ((origin = 'execution') = (execution_id IS NOT NULL)),
  CHECK ((origin = 'human') = (human_action IS NOT NULL)),
  CHECK ((origin = 'human') = (actor IS NOT NULL))
) STRICT;

CREATE TABLE revision_file (
  revision_id TEXT NOT NULL REFERENCES artifact_revision(id) ON DELETE CASCADE,
  rel_path    TEXT NOT NULL CHECK (
                rel_path <> '' AND rel_path NOT LIKE '/%' AND instr(rel_path, '\') = 0
                AND ('/' || rel_path || '/') NOT LIKE '%/../%'
                AND ('/' || rel_path || '/') NOT LIKE '%/./%'
                AND ('/' || rel_path || '/') NOT LIKE '%//%'),
  blob_sha256 TEXT NOT NULL REFERENCES blob(sha256),
  PRIMARY KEY (revision_id, rel_path)
) STRICT;
CREATE INDEX revision_file_by_blob ON revision_file(blob_sha256);

CREATE TABLE approval (
  id                 TEXT PRIMARY KEY,
  revision_id        TEXT NOT NULL UNIQUE REFERENCES artifact_revision(id) ON DELETE CASCADE,
  identity_sha256    TEXT NOT NULL CHECK (length(identity_sha256) = 64),
  gate_step_run_id   TEXT REFERENCES step_run(id) ON DELETE CASCADE,
  decision           TEXT NOT NULL CHECK (decision IN ('approve', 'reject')),
  reason_revision_id TEXT REFERENCES artifact_revision(id),
  actor              TEXT NOT NULL,
  decided_at         INTEGER NOT NULL,
  CHECK ((decision = 'reject') = (reason_revision_id IS NOT NULL)),
  CHECK (decision = 'approve' OR gate_step_run_id IS NOT NULL)
) STRICT;
CREATE UNIQUE INDEX one_decision_per_gate ON approval(gate_step_run_id)
  WHERE gate_step_run_id IS NOT NULL;
CREATE INDEX approval_by_reason ON approval(reason_revision_id);

CREATE TABLE working_copy_file (
  artifact_id         TEXT NOT NULL REFERENCES artifact(id) ON DELETE CASCADE,
  rel_path            TEXT NOT NULL,
  sha256              TEXT NOT NULL CHECK (length(sha256) = 64),
  revision_id         TEXT NOT NULL REFERENCES artifact_revision(id) ON DELETE CASCADE,
  pending_sha256      TEXT CHECK (length(pending_sha256) = 64),
  pending_revision_id TEXT REFERENCES artifact_revision(id) ON DELETE CASCADE,
  PRIMARY KEY (artifact_id, rel_path),
  CHECK ((pending_sha256 IS NULL) = (pending_revision_id IS NULL))
) STRICT;
CREATE INDEX working_copy_file_by_revision ON working_copy_file(revision_id);
CREATE INDEX working_copy_file_by_pending_revision ON working_copy_file(pending_revision_id);

CREATE TABLE execution_input (
  execution_id TEXT NOT NULL REFERENCES execution(id) ON DELETE CASCADE,
  input_name   TEXT NOT NULL,
  revision_id  TEXT NOT NULL REFERENCES artifact_revision(id),
  item_key     TEXT,
  PRIMARY KEY (execution_id, input_name),
  CHECK ((input_name = 'item') = (item_key IS NOT NULL))
) STRICT;
CREATE INDEX execution_input_by_revision ON execution_input(revision_id);

CREATE TABLE run_branch (
  workflow_run_id TEXT PRIMARY KEY REFERENCES workflow_run(id) ON DELETE CASCADE,
  ref_name        TEXT NOT NULL UNIQUE,
  start_sha       TEXT NOT NULL,
  tip_sha         TEXT NOT NULL,
  worktree_path   TEXT,
  merged_as_sha   TEXT,
  created_at      INTEGER NOT NULL
) STRICT;

CREATE TABLE ref_move (
  step_run_id   TEXT PRIMARY KEY REFERENCES step_run(id) ON DELETE CASCADE,
  ref_name      TEXT NOT NULL,
  old_sha       TEXT NOT NULL,
  new_sha       TEXT NOT NULL,
  worktree_path TEXT,
  created_at    INTEGER NOT NULL,
  CHECK (old_sha <> new_sha)
) STRICT;

CREATE TABLE event (
  seq             INTEGER PRIMARY KEY,
  at              INTEGER NOT NULL,
  root_run_id     TEXT NOT NULL REFERENCES workflow_run(id) ON DELETE CASCADE,
  workflow_run_id TEXT REFERENCES workflow_run(id) ON DELETE CASCADE,
  step_run_id     TEXT REFERENCES step_run(id) ON DELETE CASCADE,
  execution_id    TEXT REFERENCES execution(id) ON DELETE CASCADE,
  kind            TEXT NOT NULL,
  data            TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(data))
) STRICT;
CREATE INDEX event_by_root ON event(root_run_id, seq);
CREATE INDEX event_by_run ON event(workflow_run_id);
CREATE INDEX event_by_step_run ON event(step_run_id);
CREATE INDEX event_by_execution ON event(execution_id);

CREATE TRIGGER event_append_only BEFORE UPDATE ON event
BEGIN
  SELECT RAISE(ABORT, 'event rows are append-only');
END;
