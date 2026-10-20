# 03: Migration `0002` accepts input steps and answers

Spec: [Milestone 6](../spec.md) (Database) · Plan: [D. Database](../../../docs/architecture/D-database.md), [ADR 0002](../../../docs/adr/0002-file-locks-over-sqlite-leases.md)

**What to build:** the first migration after `0001`. `step_run.kind` accepts `'input'` and `artifact_revision.human_action` accepts `'answer'`, with every existing row and foreign key preserved.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] `crates/surt-engine/migrations/0002_input_steps.sql` starts with `-- no-transaction`, sets `PRAGMA foreign_keys = OFF`, rebuilds `step_run` and `artifact_revision` (new table, copy, drop, rename the new one, recreate every index) inside `BEGIN`/`COMMIT`, runs `PRAGMA foreign_key_check` before committing, and ends with `PRAGMA foreign_keys = ON`; never renames the old table with foreign keys on
- [ ] Every other CHECK, STRICT and default of both tables is unchanged; confirm sqlx 0.9 honours `-- no-transaction` and that the connection goes back to the pool with foreign keys on
- [ ] `docs/architecture/migrations/0002_input_steps.sql` is the byte-for-byte copy the store test compares against
- [ ] Store tests: a database created by `0001` with rows in every table (cascading children included) migrates with every row intact, `integrity_check` ok and `foreign_key_check` empty; an `input` step row and an `answer` Revision insert afterwards; the migration count, `PLAN_SCHEMA` and `SchemaTooNew` expectations follow
- [ ] D documents `0002` and the new values
