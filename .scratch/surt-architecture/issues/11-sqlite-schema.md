# SQLite schema

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Blocked by: 01, 03, 05, 06, 08
Assignee: Kaliel

## Question

What is the complete physical schema (7 tables) that serves any workflow and preserves referential integrity without JSON for critical relationships?

Decide: columns, types, UUIDv7 PK, FKs and `ON DELETE`; what `deleted_at` means in each table (including `events`, which should be append-only); status constraints (CHECK) and uniqueness (e.g. one active execution per step; `(workflow_run_id, logical_key)`; `(step_run_id, iteration, attempt)`); `step_dependencies` and `execution_artifacts` (input/output role); artifact revisions and hashes; where approval, lease/heartbeat and `external_ref` live; indexes for the scheduler and for `surt status`; migration strategy; what stays in JSON (snapshots, payloads) and what must not.

Output: section D with SQL that runs in `sqlite3`, a Mermaid ERD and the initial migration.

## Context

From [Agent contract](07-agent-contract.md): persist snapshots of agents/profiles/instructions/schemas and the profile chosen per Execution, without credentials. An Item List Revision covers the index and the referenced files; children point to the same Revision and its files, with no new origin and no Task-specific tables. The physical format and addressing of the bytes belong to the artifacts ticket; this ticket must represent those relationships. See the [canonical contract](../../../docs/architecture/F-agent-contract.md).

Arose from [SQLx + SQLite in practice](03-sqlx-sqlite-in-practice.md) — decide here: UUID as BLOB vs. TEXT; canonical timestamp format (fixed ISO TEXT vs. INTEGER ms) applied the same way by Rust and by SQL defaults; STRICT tables; migration policy (LF forced via `.gitattributes`, add-only, `-- no-transaction` for rebuilds, behavior of an old binary with a new DB); who enables WAL and when, at DB creation; `query!` with `.sqlx` vs. runtime `query_as`.

From [Domain glossary](01-domain-glossary.md): the suggested uniqueness `(step_run_id, iteration, attempt)` no longer holds — Iteration belongs to the **Step Run** (new Step Runs per round), Attempt to the Execution. The schema needs: a parent→child relationship between Workflow Runs (with the Step Run that started the child); a representation of the **Subject** with ≤1 active root run; Approval as its own record pointing to an Artifact Revision; a single origin per Revision (one Execution or one human edit); Execution Inputs/Outputs to Revisions.

From [Process model and concurrency control](05-process-and-concurrency-model.md): the root run needs the Driver's identity (pid, native start time, boot id), informational only, and a cancel request; each agent Execution needs the process identity and the occupied slot (`agent-<i>`), also informational only. No lease/heartbeat: ownership is decided by a file lock. Migrations run under `db.lock`; a DB with a schema newer than the binary is refused.

From [Workflow TOML schema](06-workflow-toml-schema.md): artifacts are identified by **name within a Workflow Run** (uniqueness `(workflow_run_id, name)`), and inputs resolve to the latest Revision when the Execution starts (record the fixed Revision per Execution). Two engine inputs: `request` (a human-origin Revision in the root) and `item` (a reference to an item, by `key`, of a parent's Item List Revision — not a new Revision). The reason for `surt reject` is a human Revision of the `rejection_artifact`. The root run keeps the frozen snapshot of the definition closure (`feature` + `task-cycle`) with the SHA-256 of the canonical form; Child Runs reference the root's snapshot. Step Runs belong to an Iteration of a Loop (Iteration number on the Step Run); SKIPPED state.

From [Artifact storage, freezing and revalidation](08-artifact-storage-freezing-revalidation.md): Revision bytes live in SQLite as content-addressed blobs (`blobs(sha256, bytes)` + per-Revision files; sketch tested in [H. Artifact Storage](../../../docs/architecture/H-artifact-storage.md)); publishing a Revision = one transaction. Still missing here: Revision tables (identity = SHA-256 of the `(rel_path, sha256)` pairs), Approval (revision_id + hash), a per-file **baseline** for the Working Copy, the **Pinned Revision** mark (first read as an Input after approval) and Execution directories.

From [Git integration model](09-git-integration-model.md) ([A. Git Integration](../../../docs/architecture/A-git-integration.md)): persist the `project_id` (`<dir-slug>-<6hex>`, generated when `surt.db` is created); per Task Child Run: the worktree's absolute path, the Task Branch name and the `start` SHA; per root run: the Integration Branch name (with `.N` from the Subject's second run on, so the run's ordinal within the Subject must exist), the recorded tip (for the CAS and to detect external movement) and the frozen project config (`[[checks]]`, `[git] merge_into_base`). `base` is an Initial Artifact (a human-origin Revision created together with `request` when the root run is created). Operations are Executions: the fixed `commit-tree` dates = the Execution's recorded start.

From [Builder ↔ Reviewer loop state machine](10-builder-reviewer-loop.md) ([G. State Machines](../../../docs/architecture/G-state-machines.md)): states with CHECK — Execution `queued|running|succeeded|failed|interrupted|cancelled`; Step Run `pending|running|done|skipped|blocked|cancelled` (`blocked` is not final); Workflow Run records only `active|succeeded|cancelled` + a cancel request (when, by whom), and `RUNNING/AWAITING APPROVAL/BLOCKED/INTERRUPTED` are derived on read (indexes for that). Attempt and Iteration counters keep the full history, but the budget is measured since the last unblock (record when the Step Run went back to `running`). `max_attempts`/`max_iterations` are part of the project config frozen in the root run. **There is now a single Agent Profile per Agent** (no `default_profile`, no named profiles): the agent snapshot contains one profile and the Execution only references that snapshot.

## Answer

Resolved by human exchange over two rounds, plus revisions requested on complete schema examples. Complete section in [D. Database Schema](../../../docs/architecture/D-database.md); initial migration tested in [`migrations/0001_initial.sql`](../../../docs/architecture/migrations/0001_initial.sql); trade-off in [ADR 0009 — History is purged, not soft-deleted](../../../docs/adr/0009-history-is-purged-not-soft-deleted.md).

Final decisions:

- **17 tables, singular names**, not the brief's 7: `project`, `blob`, `snapshot`, `snapshot_file`, `run_snapshot`, `subject`, `workflow_run`, `step_run`, `execution`, `execution_input`, `artifact`, `artifact_revision`, `revision_file`, `approval`, `working_copy_file`, `run_branch`, `event`. `step_dependencies` does not exist (the `needs` come from the frozen definition); `execution_artifacts` became `execution_input` (output = the Revision's origin).
- **Representation:** UUIDv7 as hyphenated TEXT; 24-character UTC TEXT timestamps (`…T14:02:11.483Z`) always written by Rust, no `CURRENT_TIMESTAMP`; `event.seq` INTEGER autoincrement; everything `STRICT`.
- **No `deleted_at`:** purge = one cascading `DELETE FROM subject` + GC of orphaned snapshots and blobs. Composition as `CASCADE`, cross-references as `NO ACTION` (tested: `RESTRICT` would break the cascade). `event` append-only via trigger (ADR 0009).
- **Content-addressed snapshots** for workflow, agent and project config, shared across runs; `run_snapshot` records each root run's closure.
- **`commit-tree` uses `step_run.started_at`**, not the Execution's start: section A said a new Attempt recomputed the same `M`, which was false with per-Execution dates. [A. Git Integration](../../../docs/architecture/A-git-integration.md#integration) corrected.
- **Connection:** WAL enabled at creation under `db.lock`, `synchronous = FULL` (the intention record must not vanish after the effect), `BEGIN IMMEDIATE`. Runtime `query_as` queries, no `.sqlx`; embedded migrations, LF forced, never edited.
- **Step Run:** `budget_base` (failures or Iterations counted at the last unblock) and `blocked_reason` ∈ `attempts_exhausted | not_retryable | iterations_exhausted`; the detail comes from the last Execution's `error_message`.
- **Approval:** one decision per Revision; `gate_step_run_id` NULL for a revalidation approval; reject requires the reason's Revision. `actor` = `git user.email`, falling back to the system user.
- **Pin:** only `pinned_at`, written in the transaction that fixes the first Input or expands the Item List (the expansion pins before any Execution exists); reason in an event.
- **Typed `run_branch`** (one branch per run, unique `ref_name`, `tip_sha` for CAS, `worktree_path`, `merged_as_sha`).
- **JSON** only in `execution.runtime_info` and `event.data`, which no engine decision reads. `event` is a transactional audit, never a source of truth, with an open `kind`.
- **`project`:** `project_id` (`<slug>-<6hex>`, human-readable because it names `~/.surt/worktrees/<project-id>/`) is the PK; the `project_single_row` trigger refuses a second row. No `singleton` column.
- **When rows are born:** top-level Step Runs are born `pending` with the run; a Loop's body ones, at the start of each Iteration; child runs, all at the Per-Item Step expansion (together with the pin); `artifact` is only born with its first Revision.
- **Purge tested end to end:** `snapshot_file → snapshot` is composition (`CASCADE`), so GC of orphaned snapshots works; `run_snapshot → snapshot` stays `NO ACTION`, so a snapshot in use cannot be deleted.
- Root-only columns stay in `workflow_run` with CHECKs (the ≤1 active run per Subject index needs `state` in the same table). **`external_ref` was not reserved**: it comes in via an additive migration when there is a provider.

Validation: the migration runs in `sqlite3` 3.51 with `-bail`; the whole flow, from `surt init` to purge, was written as executable SQL and run against it (Revisions, snapshots and blobs with real hashes checked); a full cycle (failed Attempt, Revision, reject with reason, approval, revalidation, pin, child with a blocked branch), 7 expected constraint refusals, purge with clean `foreign_key_check` and `integrity_check`. Context notes added to [Recovery and reconciliation strategy](12-recovery-and-reconciliation.md), [Cleanup and lifecycle safety](13-cleanup-delete-and-logs.md), [CLI surface and UX](14-cli-surface-and-ux.md), [Rust workspace structure and core traits](15-rust-workspace-and-traits.md) and [Test strategy](16-test-strategy.md). No new ticket.

Review against `rules/` (2026-10-17): timestamps became `INTEGER` Unix seconds (rule `sqlite.md`; order within a second comes from `seq`/`attempt`/`number`, and Git dates commits in seconds); `event.seq` lost its `AUTOINCREMENT` (rowid already guarantees order among existing rows); the migration was left without comments (rule `clean-code.md`); backup only via `VACUUM INTO` or the backup API; UUIDv7 `TEXT` kept as an exception recorded in [D](../../../docs/architecture/D-database.md#rule-exceptions). Migration revalidated in `sqlite3` 3.51 with `-bail`, `integrity_check` ok.
