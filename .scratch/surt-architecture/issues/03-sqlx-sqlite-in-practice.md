# SQLx + SQLite in practice

Map: [Surt — Architecture and implementation plan](../map.md)
Type: research
Status: resolved

## Question

What are the constraints and recommended practices for using SQLx with SQLite embedded in a distributable CLI binary?

Survey: UUIDv7 storage (16-byte BLOB × TEXT) and SQLx/`uuid` support for each, ordering and indexes; `sqlx::migrate!` embedded in the binary and behavior with already-applied migrations/checksums; `query!`/`query_as!` macros require `DATABASE_URL`/offline mode (`.sqlx`) — build cost and alternatives; per-connection pragmas (`foreign_keys`, `journal_mode=WAL`, `busy_timeout`, `synchronous`) via `SqliteConnectOptions`; write concurrency (a single writer) with Tokio and several simultaneous `surt` processes; `BEGIN IMMEDIATE` and transactions; UTC timestamps (ISO-8601 TEXT × INTEGER) and available functions; CHECK constraints, partial indexes, `STRICT` tables, generated columns, JSON1; WAL on network filesystems/Windows; binary size and SQLite bundling (`libsqlite3-sys` bundled).

Findings: `docs/research/sqlx-sqlite.md` on the `research/sqlx-sqlite` branch.

## Answer

Full findings: `docs/research/sqlx-sqlite.md` on the `research/sqlx-sqlite` branch (commit `6d3cf59`; sqlx **0.9.0**, MSRV 1.94; the repo moved to `transact-rs/sqlx`).

Facts that constrain the design (not decisions):
- The `sqlite` feature bundles SQLite statically (requires a C toolchain at build time); range 3.46.0–3.51.3, pinnable via `Cargo.lock`. STRICT, RETURNING, generated columns, partial indexes, JSON and `unixepoch`/`subsec` are available.
- `uuid::Uuid` maps to BLOB(16); TEXT requires `fmt::Hyphenated`. Both sort by time for UUIDv7. `now_v7()` ordering is only guaranteed within one process.
- sqlx enables `foreign_keys` and `busy_timeout=5s` by default; it does **not** enable WAL or `synchronous`; `create_if_missing=false`. The first switch to WAL requires an exclusive lock and can collide across processes.
- `begin()` is DEFERRED → read-then-write can fail immediately with SQLITE_BUSY/BUSY_SNAPSHOT, ignoring the busy timeout; `begin_with("BEGIN IMMEDIATE")` exists; BUSY arrives as `ErrorKind::Other` (retry must match on `code()`).
- The migration lock on SQLite is a no-op → two processes migrating at the same time rely only on SQLite's locking (likely an error in one of them; not tested).
- The migration checksum is SHA-384 of the bytes → CRLF × LF breaks it (`VersionMismatch`; a real case in openai/codex). An old binary with a new DB fails (`VersionMissing`). Migrations run in a transaction where `PRAGMA foreign_keys=OFF` does not work — a table rebuild requires `-- no-transaction`/`defer_foreign_keys`.
- Timestamps: chrono writes `...T...+00:00` with variable fractions; `CURRENT_TIMESTAMP` writes `YYYY-MM-DD HH:MM:SS` — they do not compare as text. INTEGER is read as seconds (ms requires a manual `i64`). `ADD COLUMN` does not accept an expression default.
- `query!` requires a committed `.sqlx` + `cargo sqlx prepare --check` in CI (or a DB at build time); a stray `DATABASE_URL` wins over `.sqlx`. On STRICT tables the macros infer `Vec<u8>`/`String`/`i64` and need overrides. `query_as::<_, T>` + `FromRow` at runtime avoids build coupling.
- WAL: same host, writable directory, `-wal`/`-shm` travel with the DB; WAL grows without bound if checkpoints are starved.
- One writer at a time; each pool connection is a thread. `#[sqlx::test]` gives a fresh file DB per test with migrations; in-memory shared-cache does not reproduce WAL/multi-process.

Open: binary size/compile time (measure), real concurrent migration (experiment), `time` format (`Z` × `+00:00`).
