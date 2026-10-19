# A. Process Model and Concurrency

Part of section A (Architecture); the component overview is [A. Overview](A-overview.md). It covers who advances a Workflow Run, for how long, and how Surt prevents duplicate work between processes. Vocabulary (**Driver**, **Interruption**, **Cancellation**) is defined in [`CONTEXT.md`](../../CONTEXT.md). Decision ticket: [Process model and concurrency control](../../.scratch/surt-architecture/issues/05-process-and-concurrency-model.md).

## Driving a run

The engine has one primitive: `drive(root_run_id)`. It advances a root Workflow Run and all its Child Workflow Runs until nothing is left that can run without a human. That means every non-terminal Step Run is waiting on an Approval Gate or is blocked ([G. State Machines](G-state-machines.md#when-a-driver-returns)), or the run is terminal. Then it returns. A root Workflow Run has **at most one Driver**. Child Workflow Runs are always driven by their root's Driver, never on their own.

`drive` keeps no global state. All state it needs to continue lives in SQLite and on disk, so any Driver can pick up a run that another Driver left, whether it stopped cleanly or not.

### Commands of the built-in feature workflow

Each command owns one phase. A command that drives runs in the foreground. It refuses with an actionable message when its precondition is not met. Running the same command again continues from where it stopped. There is no separate `resume` command in v1.

| Command | Drives? | Precondition | Runs until |
|---|---|---|---|
| `surt run feature --input request="<what to build>"` | yes | no active run for a Subject of the same slug | spec Approval Gate |
| `surt approve <feature>` | **no**, it only records the Approval | an Approval Gate is waiting | returns immediately |
| `surt plan <feature>` | yes | spec Revision approved | task-manifest Approval Gate |
| `surt start <feature>` | yes | task-manifest Revision approved | Shipper delivers the final integrated branch, then the opt-in fast-forward into the Base runs ([A. Git Integration](A-git-integration.md)) (run terminal) |
| `surt stop <feature>` | no (see [External control](#external-control)) | a Driver is live | Interruption requested |
| `surt cancel <feature>` | no (see [External control](#external-control)) | run not terminal | Cancellation recorded |

```
$ surt start 20261015-add-login
✖ tasks not approved yet (manifest rev 1 awaiting approval)
  review manifest rev 1 and its Task Markdown files, then: surt approve 20261015-add-login
```

All three driving commands call the same `drive`. The phase boundaries come from the Approval Gates declared in the workflow, not from engine knowledge of specs or tasks. `surt approve --no-run` does not exist, because `approve` never drives.

## Ownership: one Driver per root run

Ownership is an exclusive OS file lock (`std::fs::File::lock`, Rust ≥ 1.89) on a separate file per root run:

```
.surt/runtime/locks/<root-run-id>.lock
```

- A driving command takes the lock with `try_lock` before doing anything. If the lock is taken, it refuses at once:
  ```
  ✖ run 20261015-add-login is being driven by pid 48213 (started 14:02)
    stop it with Ctrl-C in its terminal, or: surt stop 20261015-add-login
  ```
- The OS releases the lock when the process dies (crash, `kill -9`, reboot). There is no heartbeat and no time-based expiry. A Driver that sleeps for hours with the laptop lid closed still owns its run when it wakes.
- SQLite records the Driver's identity (pid, native start time, boot id) on the root run, **for display and recovery only**. The lock decides. The row never does.
- Executions carry no heartbeat. An Execution in a running state whose root run has no live Driver (its lock is free) is an orphan by definition. See [Orphaned agents](#orphaned-agents).

Rationale and rejected alternatives: [ADR 0002](../adr/0002-file-locks-over-sqlite-leases.md).

## Interruption (Ctrl-C) and Cancellation

**Interruption** stops the Driver without ending the run:

1. The first Ctrl-C (or SIGHUP when the terminal closes, or the console closing on Windows) starts a graceful stop of every running agent Execution, in parallel (see [Supervising agent processes](#supervising-agent-processes)).
2. A **Operation already in progress is never cut** by the first interruption. The Driver waits for it to finish, because Operations are the intention → effect points that recovery would otherwise have to reconcile.
3. Stopped agent Executions are recorded as `interrupted` ([G. State Machines](G-state-machines.md#execution)); this does not count toward the Attempt limit. The run stays resumable, and the next driving command creates new Attempts for them in fresh Claude sessions. The work of an interrupted session is lost. Its worktree may be dirty: the next Attempt salvages and resets it ([I. Recovery](I-recovery.md#preparing-a-worktree-for-every-attempt)).
4. A second Ctrl-C forces it: the agent process trees are killed and the Driver exits at once, even in the middle of a Operation. Recovery reconciles the partial state.

```
^C stopping 2 executions… finishing integrate (task B)… (Ctrl-C again to force)
✔ stopped. resumable: surt start 20261015-add-login
```

Agent processes run in their own process group (Unix) or with `CREATE_NEW_PROCESS_GROUP` (Windows). So the terminal's Ctrl-C reaches only the Driver, which then directs the stop.

**Cancellation** is always explicit (`surt cancel`). It ends the run in a terminal state. Work starts again only as a new Workflow Run of the same Subject.

## External control

Every control action from another process goes **through SQLite, never through signals between Surt processes**:

- `surt stop` writes a stop request on the root run. A live Driver reads it on its next tick and performs the same Interruption as a first Ctrl-C; the run stays resumable ([J](J-cli.md#stop-and-cancel)).
- `surt cancel` writes a cancel request on the root run. A live Driver reads it on its next scheduler tick (about 1 s), stops its Executions as in an Interruption, and records the Cancellation. If no Driver is live (the lock is free), `cancel` takes the lock itself and records the Cancellation directly.
  ```
  T2 $ surt cancel 20261015-add-login
  ⏳ cancel requested; driver pid 48213 is stopping 3 executions…
  ✔ run cancelled
  T1   ✖ cancelled by surt cancel (from another terminal) — 3 executions stopped
  ```
- An Approval recorded while a Driver is still busy on another branch of the DAG is picked up on the next tick through the same channel. The feature workflow never hits this case, but generic workflows can.

## Agent concurrency limit

- **Scope: per project.** Surt can run in several projects on the same machine, and each project has its own budget. Shared machine-wide pressure (account rate limits, token cost) is not handled by this limit. It stays open as fog on the map.
- **Configuration:** `max_concurrent_agents`, default `3`, set in the project configuration ([J. Configuration](J-cli.md#configuration)).
- **What counts:** only agent Executions. Operations (Git commands, build or test steps) do not take a slot.
- **Mechanism: slots are lock files.**
  ```
  .surt/runtime/slots/agent-0.lock … agent-{N-1}.lock
  ```
  On each tick, every Driver of the project tries `try_lock` on each slot below the limit that it does not already hold, and launches agents only into the slots it obtained. A slot taken for a launch is held until that agent Execution ends; slots tried but not used are released in the same tick. If none is free, the agent stays queued (no Execution is created yet) and the Driver retries on its next tick, about one second later. The directory is created on first use and the lock files are never deleted (`surt delete` leaves them alone: they belong to the project, not to a Subject). A Driver that dies frees its slots at once through the OS. SQLite records which slot an Execution holds, for display only (`status` shows `4/4 in use`).
- **Live limit.** Every running Driver rereads `max_concurrent_agents` from `.surt/config.toml` before each tick, so a developer can throttle or speed up a long run without stopping it. Only this key is read live; the frozen keys of the `config` snapshot stay frozen ([J. Configuration](J-cli.md#configuration)).
  - A lowered limit stops offering the slots at or above it. Agents already holding them finish; nothing is stopped.
  - A raised limit offers the new slots on the next tick.
  - A missing file keeps the limit in force.
  - **Warning (decided here).** If the file stops parsing while a Driver runs, the Driver keeps its last valid limit and reports one warning progress event, printed on stderr. The warning repeats only after the file has parsed again and then broken again. The run never fails over it; the next command that opens the project refuses with the usual invalid-configuration message.
  ```
  ⚠ invalid configuration in .surt/config.toml:1 — keeping max_concurrent_agents = 2
    string values must be quoted
    fix: edit .surt/config.toml:1
  ```
- **Fairness:** none across processes in v1. Whichever Driver tries first takes the free slot. Inside one Driver, queued Executions go in a deterministic order (manifest order for Tasks). Starvation is bounded by the duration of one agent run. A shared FIFO can be added later without changing the model.

```
B  build #1   QUEUED   waiting for agent slot (4/4 in use)
```

## Native Git operations across processes

Different Features in the same repository can run Operations at the same moment, for example two `git worktree add` calls. Git's own lock files (`index.lock`, `<ref>.lock`, `packed-refs.lock`) would make one of them fail. So Surt serializes its own Operations on shared repository state under one lock:

```
.surt/runtime/locks/git.lock
```

It is held for worktree add or remove, branch creation, `update-ref`, and `merge-tree`/`commit-tree`. It also retries with backoff on Git lock failures caused by third parties (an IDE, the user in a terminal). Commits a Builder makes inside its own worktree do not go through this lock, because Surt does not control the agent's Git. They touch distinct refs, and a rare collision falls into the retry or a new Attempt.

## Supervising agent processes

- Agent processes are spawned with **`process-wrap` 10.x**: a process group on Unix, a Job Object on Windows. The kill always targets the whole tree, never only the direct child (`kill_on_drop`/`Child::kill` reach only the direct child).
- `process-wrap` does not escalate by itself, so Surt implements the escalation:
  1. **Graceful:** SIGTERM to the group on Unix, `CTRL_BREAK_EVENT` to the group on Windows. Claude Code exits 143 on SIGTERM, kills its Bash trees and runs `SessionEnd` hooks.
  2. **Grace period:** `stop_grace_period`, default `10s`, configurable in the global configuration.
  3. **Forced:** kill the process group, or terminate the Job Object.
- Surt imposes no wall-clock execution timeout in the current plan, as decided in [F. Agent Contract](F-agent-contract.md). The grace period above only bounds an explicit stop sequence; it is not an execution duration limit.
- To reduce orphans, Windows uses a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, so children die with the Driver. Linux sets `PR_SET_PDEATHSIG` on the direct child. macOS has no equivalent and relies on the next Driver (see below).
- Each agent Execution records its process identity: pid, native start time and boot id. A reused pid is never mistaken for the original process.
- **Unverified on real Windows:** how `claude.exe` reacts to Ctrl+Break. If it ignores it, the only effect is that the forced kill comes after the grace period.

## Orphaned agents

On Unix, an agent's process group can outlive a Driver killed with `kill -9`. The agent keeps editing its worktree while nobody reads its stream. **Surt never adopts orphans.** When a driving command finds an Execution in a running state with no live Driver, it checks the recorded identity. If that process still exists, Surt stops its group with the same escalation, and only then creates the next Attempt:

```
⚠ found orphan builder for task B (pid 50122, alive) — stopping it
↻ task B build #1: attempt 2
```

Adoption is rejected because the orphan's stdout was a pipe to the dead Driver: its stream and final `result` are lost and its output could not be validated. [I. Recovery](I-recovery.md#stale-executions-and-orphans) handles what the orphan left behind (a dirty worktree, or a commit made but not recorded).

## Opening the database

Several `surt` processes may open the same `.surt/surt.db` at once, for example right after an upgrade:

1. Take an exclusive lock on `.surt/runtime/locks/db.lock`.
2. Configure the connection: WAL, `foreign_keys = ON`, `busy_timeout`.
3. Run the embedded migrations, only while holding the lock, then release it.
4. If the database schema is **newer** than the binary supports, refuse:
   ```
   ✖ .surt/surt.db is at schema 7; this surt supports up to 5 — upgrade surt
   ```

Every write transaction afterwards uses `BEGIN IMMEDIATE` and retries on `SQLITE_BUSY`. SQLite allows only one writer at a time across processes.

All lock files live apart from `surt.db`, `-wal` and `-shm`. SQLite uses POSIX locks, which an unrelated `close()` on the same file would drop.

## Lock and runtime file layout

```
.surt/runtime/
├── locks/
│   ├── <root-run-id>.lock   # ownership of one root Workflow Run
│   ├── git.lock             # Surt's Operations on shared repo state
│   └── db.lock              # migrations at database open
└── slots/
    └── agent-<i>.lock       # agent concurrency slots, i in 0..max_concurrent_agents
```

`.surt/runtime/` also holds one directory per Execution (`executions/<id>/{inputs,delivery}/`). Logs are not runtime state: they live in `.surt/logs/<subject>/<execution-id>/` ([I. Cleanup](I-cleanup.md#logs)). It is ignored by Git with the rest of `.surt/` except `config.toml` and `agents/`; see [H. Artifact Storage, Freezing and Revalidation](H-artifact-storage.md).

## Daemon fit

A local daemon is out of scope, but nothing here prevents one. These rules hold for every later design:

1. **The Driver is a library function**, `drive(root_run_id)`, with no global state. CLI commands only call it. A daemon would call it N times in one process.
2. **No state lives only in memory between commands.** Everything needed to continue is in SQLite and on disk.
3. **Ownership and capacity are file locks** (run, slots, git, db). A daemon would hold the same locks, so CLI and daemon coexist without knowing about each other.
4. **External control goes only through SQLite** (cancel, approve), never through signals.
5. **Agents are never adopted.** An agent that outlives its Driver is stopped and relaunched as a new Attempt, so a daemon never needs to reattach processes.

What a daemon would add (`--detach`, and a live log channel for clients) is out of scope.

## Decided elsewhere

- Execution, Step Run and Workflow Run state names (interrupted, queued, cancel requested): [G. State Machines](G-state-machines.md).
- What recovery does with orphans' leftovers and partial Operations, and what `cancel` and `status` do without a Driver: [I. Recovery and Reconciliation](I-recovery.md).
- Columns for the Driver identity, cancel request and slot held: [D. Database Schema](D-database.md#workflow-runs).
- The Agent Profile, permissions and the decision to omit execution timeouts: section F.
- Full CLI contracts, output formats and exit codes: [J. CLI Surface](J-cli.md).
