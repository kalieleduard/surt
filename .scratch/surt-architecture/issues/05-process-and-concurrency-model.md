# Process model and concurrency control

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Blocked by: 01, 18
Assignee: Kaliel

## Question

Who executes the workflow, for how long, and how does Surt prevent duplicate work across processes?

Decide: which commands "drive" the run (e.g. `surt feature` runs until `AWAITING_APPROVAL` and exits; does `surt approve` continue in the foreground?), what happens on Ctrl-C (cancel the child executions or leave them orphaned for reconciliation?), the per-workflow-run lock (file × SQLite lease × both), execution heartbeat/lease and expiry deadline, several features running in parallel in the same repo (each in its own process?), a global per-project agent concurrency limit across separate processes, how the child process (Claude Code) is supervised (process group/job object), and what in the design guarantees that a future daemon fits without rework.

## Context

**User requirement** (from [Domain glossary](01-domain-glossary.md)): it must be possible to leave N approved features and run them **simultaneously in the same repo**. The domain model already allows it (the invariant is ≤1 active Workflow Run per Subject, not per project); what remains to decide here is how the processes are organized and which global agent limit applies across them.

Came up in [Cross-platform primitives (macOS, Linux, Windows)](18-cross-platform-primitives.md): killing a tree requires a process group (Unix) / Job Object (Windows) — adopt `process-wrap` or implement it?; graceful escalation (SIGTERM → SIGKILL; Ctrl+Break on Windows) and deadlines; per-workflow lock in a separate file via `std::fs::File::lock` × SQLite lease; PID identity = native start time + boot id.

Also from [SQLx + SQLite in practice](03-sqlx-sqlite-in-practice.md): one writer at a time across processes; write transactions must be `BEGIN IMMEDIATE` with retry on BUSY; concurrent migration across processes is not protected by sqlx — serialize it (e.g. under the file lock) when opening the DB.

## Answer

Resolved in a grilling session. Document: `docs/architecture/A-process-model.md`; trade-off in `docs/adr/0002-file-locks-over-sqlite-leases.md`; terms **Driver**, **Interruption** and **Cancellation** in `CONTEXT.md`.

Decisions:
- **Who drives:** the core has a single primitive, `drive(root_run_id)`, which advances the root run and its Child Runs until only a wait on a human or the end remains. In the feature workflow, each command owns one phase: `surt feature` (up to the spec gate), `surt plan` (Task Agent, up to the tasks gate; requires an approved spec), `surt start` (builders → reviews → Shipper, up to final delivery; requires approved tasks). **`surt approve` only records the Approval, it never drives.** Running the same command again continues where it left off; no `surt resume` in M1.
- **Unit of ownership:** one process drives one root run with all its Child Runs; N features = N processes.
- **One Driver per run:** an exclusive file lock (`.surt/runtime/locks/<root-run-id>.lock`, `File::lock`) decides; SQLite stores pid + start time + boot id only for display and recovery. No lease, heartbeat or time-based expiry (a laptop going to sleep does not create a duplicate owner). A `RUNNING` Execution with the run's lock free = orphan.
- **Ctrl-C = Interruption:** graceful stop of the agents in parallel, Executions marked as interrupted, run resumable with new Attempts; a Native Operation in progress is never cut off by the first Ctrl-C; the second one forces it. SIGHUP/closing the console = same path. **Cancellation** only via `surt cancel`.
- **External control only through SQLite:** `cancel` writes a cancel request that the Driver reads on every tick (~1s); with no live Driver, `cancel` itself takes the lock and cancels. Never through signals between Surt processes.
- **Per-project agent limit** (`max_concurrent_agents`, default 3), because Surt runs in several projects on the same machine; only agent Executions count. Mechanism: slots = lock files `.surt/runtime/slots/agent-<i>.lock` with `try_lock` and ~500ms polling; no fairness across processes in M1.
- **Native Git serialized per repo** under `.surt/runtime/locks/git.lock`, plus retry with backoff for Git locks caused by third parties; Builders' commits inside their worktree are excluded.
- **Supervision:** `process-wrap` 10.x (process group / Job Object) with our own escalation: SIGTERM or `CTRL_BREAK_EVENT` → `stop_grace_period` (default 10s, global config) → kill the tree. `KILL_ON_JOB_CLOSE` on Windows and `PR_SET_PDEATHSIG` on Linux. The original plan for a per-Agent-Profile timeout was withdrawn by a later human decision in [Agent contract](07-agent-contract.md), round 2; there is no execution timeout in the current plan.
- **Orphans are never adopted:** the next driving command checks the recorded identity, kills the surviving group and only then creates the new Attempt.
- **Opening the DB:** migrations only under `.surt/runtime/locks/db.lock`; WAL + `foreign_keys` + `busy_timeout`; `BEGIN IMMEDIATE` with retry; refuses a DB whose schema is newer than the binary.
- **Future daemon:** five principles recorded (Driver as a library function, no memory-only state, file locks, control only through SQLite, no adoption of agents).
