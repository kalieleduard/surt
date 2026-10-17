# 02: Store opens the schema

Spec: [Milestone 1](../spec.md) · Slice: S1 Skeleton · Plan: [D](../../../docs/architecture/D-database.md), [B. Store interface](../../../docs/architecture/B-workspace.md#the-store-interface), [ADR 0002](../../../docs/adr/0002-file-locks-over-sqlite-leases.md)

**What to build:** the engine can open a project database that is the source of truth: the plan's `0001_initial.sql` applied unchanged, safe against concurrent openers and against a binary older than the database, with the transaction discipline every later checkpoint will use. The cross-platform file lock of `platform` lands here, because migrations run under `db.lock`.

**Blocked by:** 01 (Workspace, toolchain and CI matrix)

**Status:** ready-for-agent

- [x] The migration is a byte-for-byte copy of the plan's `0001_initial.sql` (17 STRICT tables, UUIDv7 TEXT ids, INTEGER Unix-second timestamps written by Rust), immune to CRLF checkout on Windows
- [x] Opening sets WAL and `synchronous=FULL`, and runs migrations under `db.lock` via `File::lock`; every OS `#[cfg]` lives in `platform`
- [x] A database whose schema is newer than the binary is refused with `SchemaTooNew`
- [x] All SQL lives in the single store module; writes go through `BEGIN IMMEDIATE` with retry on `SQLITE_BUSY`, and no transaction handle leaves the module
- [x] Tests against a file-backed SQLite in a tempdir prove the invariants are enforced by the schema: at most one active root run per Subject and at most one live Execution per Step Run
- [x] Tests cover a fresh open, a reopen, and the `SchemaTooNew` refusal
