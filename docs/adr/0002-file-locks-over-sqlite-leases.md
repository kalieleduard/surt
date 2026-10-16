# File locks, not SQLite leases, decide who owns a run

Ownership of a root Workflow Run, the per-project agent slots, Surt's Git Native Operations and database migrations are all guarded by exclusive OS file locks (`std::fs::File::lock`) on files under `.surt/runtime/`. They are not guarded by leases with heartbeats in SQLite. The OS releases a file lock exactly when its holder dies and never while it lives. A lease, by contrast, expires on wall-clock time. A laptop that sleeps for hours lets another process take over while the original Driver still believes it owns the run, and that produces duplicate Builders unless every write is fenced. SQLite still records the holder's pid, start time and boot id, but only for display and recovery. The lock decides.

## Considered Options

- **Lease + heartbeat in SQLite:** works without file-lock support, but needs an expiry period, fencing on every write, and reconciliation of stale slots after a crash.
- **Both as equals:** two sources of truth that can disagree.

## Consequences

- Lock files must live apart from `surt.db`, `-wal` and `-shm`, because SQLite's POSIX locks are dropped by any unrelated `close()` on the same file.
- Running `.surt/` on a network filesystem with unreliable locking is unsupported.
- Executions need no heartbeat. An Execution in a running state whose run lock is free is an orphan by definition.
