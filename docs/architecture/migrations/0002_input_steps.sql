-- no-transaction
PRAGMA foreign_keys = OFF;

BEGIN IMMEDIATE;

CREATE TABLE new_step_run (
  id               TEXT PRIMARY KEY,
  workflow_run_id  TEXT NOT NULL REFERENCES workflow_run(id) ON DELETE CASCADE,
  step_key         TEXT NOT NULL,
  kind             TEXT NOT NULL CHECK (kind IN ('agent', 'approval', 'operation', 'loop', 'per_item', 'input')),
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

INSERT INTO new_step_run (
  id, workflow_run_id, step_key, kind, loop_step_run_id, iteration, state,
  outcome, blocked_reason, budget_base, created_at, started_at, finished_at
)
SELECT
  id, workflow_run_id, step_key, kind, loop_step_run_id, iteration, state,
  outcome, blocked_reason, budget_base, created_at, started_at, finished_at
FROM step_run;

DROP TABLE step_run;

ALTER TABLE new_step_run RENAME TO step_run;

CREATE UNIQUE INDEX one_step_run_outside_loops ON step_run(workflow_run_id, step_key)
  WHERE loop_step_run_id IS NULL;
CREATE UNIQUE INDEX one_step_run_per_iteration ON step_run(loop_step_run_id, iteration, step_key)
  WHERE loop_step_run_id IS NOT NULL;
CREATE INDEX step_run_by_run ON step_run(workflow_run_id, state);
CREATE INDEX step_run_by_loop ON step_run(loop_step_run_id);

CREATE TABLE new_artifact_revision (
  id              TEXT PRIMARY KEY,
  artifact_id     TEXT NOT NULL REFERENCES artifact(id) ON DELETE CASCADE,
  number          INTEGER NOT NULL CHECK (number >= 1),
  identity_sha256 TEXT NOT NULL CHECK (length(identity_sha256) = 64),
  origin          TEXT NOT NULL CHECK (origin IN ('execution', 'human')),
  execution_id    TEXT REFERENCES execution(id) ON DELETE CASCADE,
  human_action    TEXT CHECK (human_action IN ('initial', 'edit', 'rejection_reason', 'answer')),
  actor           TEXT,
  pinned_at       INTEGER,
  created_at      INTEGER NOT NULL,
  UNIQUE (artifact_id, number),
  UNIQUE (execution_id, artifact_id),
  CHECK ((origin = 'execution') = (execution_id IS NOT NULL)),
  CHECK ((origin = 'human') = (human_action IS NOT NULL)),
  CHECK ((origin = 'human') = (actor IS NOT NULL))
) STRICT;

INSERT INTO new_artifact_revision (
  id, artifact_id, number, identity_sha256, origin, execution_id,
  human_action, actor, pinned_at, created_at
)
SELECT
  id, artifact_id, number, identity_sha256, origin, execution_id,
  human_action, actor, pinned_at, created_at
FROM artifact_revision;

DROP TABLE artifact_revision;

ALTER TABLE new_artifact_revision RENAME TO artifact_revision;

PRAGMA foreign_key_check;

COMMIT;

PRAGMA foreign_keys = ON;
