# B. Rust Workspace, Modules and Traits

How the code is organised into crates and modules, what each module hides, and which traits exist between them. Vocabulary is defined in [`CONTEXT.md`](../../CONTEXT.md); the module diagram is in [A. Overview](A-overview.md). Decision ticket: [Rust workspace structure and core traits](../../.scratch/surt-architecture/issues/15-rust-workspace-and-traits.md).

Rust contracts here are signatures and invariants, not bodies. Names may change during implementation; the seams and what crosses them may not without a new decision.

## Two crates

```
Cargo.toml                    # [workspace] members = ["crates/*"], resolver = "3"
rust-toolchain.toml           # pinned stable
crates/
├── surt-engine/              # lib: the generic engine
│   ├── migrations/0001_initial.sql
│   ├── schemas/              # base, check-report, conflict, integration (Operation outputs)
│   └── src/…
└── surt/                     # bin "surt": the CLI and the built-in feature workflow
    ├── assets/workflows/{feature,task-cycle}.toml
    ├── assets/agents/<agent>/{agent.toml,instructions.md,schemas/…}
    └── src/…
```

- **`surt-engine`** knows the engine layer of [C. Domain Model](C-domain-model.md#two-layers) and nothing else. It cannot import anything from `surt`, so the compiler enforces that the core never names Spec, Task, Builder, Reviewer, Shipper or a phase.
- **`surt`** owns everything that is about the feature workflow or about a terminal: clap, the phase preconditions of `plan` and `start`, the gate → next-command table, every message text, exit codes, and the embedded `feature`/`task-cycle` definitions and their five Agents.
- The feature workflow's files do not exist inside `surt-engine`. Engine tests therefore build their own synthetic workflows, which keeps the core honest about being generic.

Rationale and rejected alternatives: [ADR 0011](../adr/0011-two-crates-keep-the-engine-generic.md).

## The engine's interface: `Project`

The CLI sees one facade type and the types it returns. Everything else in `surt-engine` is `pub(crate)`.

```rust
pub struct Project { /* .surt/ root, store, config, catalog */ }

impl Project {
    /// Finds `.surt/` from `cwd`, opens the database (migrations under `db.lock`),
    /// loads project + global configuration. Refuses a database newer than the binary.
    pub async fn open(cwd: &Path, catalog: Catalog) -> Result<Project, Error>;

    /// Full name, slug or unique prefix. `Refusal::AmbiguousSubject` / `UnknownSubject`.
    pub async fn find_subject(&self, query: &str) -> Result<SubjectRef, Error>;

    /// Validates and freezes the closure of definitions and agents, writes the
    /// "run created" checkpoint. Refuses when the Subject already has an active run.
    pub async fn create_run(&self, new: NewRun) -> Result<RunId, Error>;

    /// try_lock the root run → reconcile → advance until nothing can run without
    /// a human → cleanup if terminal. The only primitive that drives.
    pub async fn drive(
        &self,
        run: RunId,
        runtime: &impl AgentRuntime,
        progress: ProgressSink,
        stop: StopSignal,
    ) -> Result<DriveOutcome, Error>;

    pub async fn approve(&self, gate: GateRef, rev: RevisionRef) -> Result<(), Error>;
    pub async fn reject(&self, gate: GateRef, rev: RevisionRef, reason: String) -> Result<(), Error>;
    /// Refuses with the run's derived state when no Driver is live.
    pub async fn request_stop(&self, subject: &str) -> Result<StopRequest, Error>;
    pub async fn cancel(&self, run: RunId) -> Result<CancelResult, Error>;
    pub async fn restore(&self, artifact: ArtifactRef) -> Result<(), Error>;
    pub async fn delete(&self, subject: SubjectRef) -> Result<DeleteResult, Error>;

    /// Read-only: never takes a lock, never reconciles, never writes.
    pub async fn status(&self, subject: Option<SubjectRef>) -> Result<StatusView, Error>;
    pub async fn inspect(&self, target: InspectRef) -> Result<InspectView, Error>;
    pub async fn show(&self, artifact: ArtifactRef, rev: Option<RevisionRef>) -> Result<RevisionView, Error>;
}
```

Invariants of the interface:

- **`drive` is the whole driving sequence.** Lock, reconciliation ([I. Recovery](I-recovery.md#what-a-driving-command-does-when-it-takes-the-lock)) and cleanup ([I. Cleanup](I-cleanup.md#when-cleanup-runs)) happen inside it, so a later daemon gets them by calling the same function ([A. Process Model](A-process-model.md#daemon-fit)).
- **Phases are not in the engine.** `surt plan` checks "spec Revision approved" on a `StatusView` before calling `drive`; the engine only sees Approval Gates declared in the frozen definition.
- **Confirmations are not in the engine.** `cancel`, `restore` and `delete` act when called; the CLI asks first, using `status`/`inspect` to show the impact.
- **Subject resolution is in the engine.** A Subject is an engine concept; how a name, slug or prefix matches is the same for every workflow.

## Modules of `surt-engine`

| Module | Hides | Dependency category |
|---|---|---|
| `project` | the facade; finding `.surt/`; project and global configuration; the `Catalog` of built-ins | — |
| `workflow` | parsing the five `kind`s with `deny_unknown_fields`; static validation against agents and the Operation registry; merging `.surt/agents/` over the `Catalog` field by field; freezing the closure and its SHA-256 ([E](E-workflow-schema.md), [F](F-agent-contract.md#built-in-defaults-and-local-overrides)) | in-process, pure |
| `drive` | the tick loop, the state machines of [G](G-state-machines.md), Iterations and Per-Item expansion, agent slots, the rate-limit pause, Interruption and Cancellation | orchestrates the others |
| `reconcile` | the partial-state tables of [I. Recovery](I-recovery.md), orphans, Salvage Refs, preparing a clean worktree for every Attempt | real Git, real processes |
| `artifacts` | blobs by SHA-256, atomic publication of Revisions, delivery validation, Working Copy, edit detection, pinning and revalidation ([H](H-artifact-storage.md)) | store + disk |
| `context` | the Context Assembler: each Execution's `runtime/executions/<id>/` inputs and read-only item files | disk |
| `runtime` | the `AgentRuntime` trait; the Claude Code adapter (launcher argv, stream-json, failure classification); the Fake Runtime; process-tree supervision and stop escalation | **the one real seam** |
| `operations` | the six Operations as a closed registry ([A. Git Integration](A-git-integration.md#operation-registry)) | real Git |
| `git` | running `git` by argv, the minimum-version check, `git.lock` with backoff on third-party Git locks | real Git |
| `cleanup` | post-terminal cleanup and `delete` ([I. Cleanup](I-cleanup.md)) | real Git + disk |
| `store` | all SQL ([below](#the-store-interface)) | real SQLite |
| `platform` | every `#[cfg]` for macOS, Linux and Windows: file locks, process identity (pid + start time + boot id), safe directory removal, portable names, `dunce` canonicalisation | OS |

`project` is the only public module, plus the types the facade exposes: `runtime::{AgentRuntime, ClaudeCodeRuntime, FakeRuntime}` and their request/response types, `Progress`, `StopSignal`, `DriveOutcome`, `Error`, `Refusal`, `Catalog` and the read views.

## Traits

There is exactly one trait across a seam: **`AgentRuntime`**. A seam earns a trait only when two adapters exist.

| Seam | Adapters | Decision |
|---|---|---|
| Agent Runtime | Claude Code + Fake Runtime | **trait** |
| Store | SQLite only; tests use a real file in a temp directory ([D](D-database.md#queries-and-migrations-in-code)) | no trait |
| Git | the `git` CLI only; tests use a real repository in a temp directory | no trait |
| Operations | six closed cases, matched on an `enum` | no trait |
| Clock | timers are tested with `tokio::time::pause()`; computations that need the wall clock (pause until a reset time) are pure functions taking `now` | no trait |

```rust
pub trait AgentRuntime: Send + Sync {
    fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> impl Future<Output = Result<RuntimeCompletion, RuntimeError>> + Send;
}
```

- Native `async` in traits, no `async-trait`. The trait is not object-safe and does not need to be: `drive` takes `&impl AgentRuntime`, so `Project` carries no type parameter.
- `ClaudeCodeRuntime` and `FakeRuntime` both live in `surt-engine::runtime` and are public. The Fake goes through the same delivery validation and never writes to SQLite ([F](F-agent-contract.md#agent-runtime-and-fake-runtime)). How a test selects the Fake for the binary is decided in [Test strategy](../../.scratch/surt-architecture/issues/16-test-strategy.md).
- `RuntimeError` carries the `FailureClass` (`transient`, `not_retryable`, `retryable`) classified by the adapter ([I. Recovery](I-recovery.md#transient-failures-network-and-rate-limits)). `drive` records it on the Execution; it never crosses the facade as an error.

Rationale: [ADR 0012](../adr/0012-agent-runtime-is-the-only-trait.md).

## The store interface

One method per transactional checkpoint of [I. Recovery](I-recovery.md#transactional-checkpoints), not one repository per table. Each method is one `BEGIN IMMEDIATE` transaction, retried on `SQLITE_BUSY`, that writes its rows and its `event` together. No transaction handle leaves the module.

```rust
impl Store {
    async fn create_run(&self, rows: NewRunRows) -> Result<RunId>;
    async fn start_iteration(&self, loop_: StepRunId, rows: IterationRows) -> Result<()>;
    async fn expand_items(&self, step: StepRunId, children: Vec<ChildRunRows>) -> Result<()>;
    async fn start_execution(&self, start: ExecutionStart) -> Result<ExecutionId>;
    async fn record_spawn(&self, exec: ExecutionId, ident: ProcessIdentity) -> Result<()>;
    async fn record_intention(&self, intention: Intention) -> Result<()>;   // run_branch or ref_move row
    async fn accept_execution(&self, acceptance: Acceptance) -> Result<()>;
    async fn end_execution(&self, exec: ExecutionId, end: ExecutionEnd) -> Result<()>;
    async fn promote_working_copy(&self, artifact: ArtifactId) -> Result<()>;
    async fn record_approval(&self, approval: NewApproval) -> Result<()>;
    async fn request_stop(&self, run: RunId) -> Result<()>;
    async fn request_cancel(&self, run: RunId) -> Result<()>;
    async fn finish_run(&self, run: RunId, state: RunState) -> Result<()>;
    async fn mark_cleaned(&self, run: RunId) -> Result<()>;
    async fn purge_subject(&self, subject: SubjectId) -> Result<()>;

    async fn load_run_graph(&self, root: RunId) -> Result<RunGraph>;   // everything the scheduler reads
    async fn status_view(&self, subject: Option<SubjectId>) -> Result<StatusView>;
}
```

`drive` **reloads the `RunGraph` on every tick** (about 1 s; a run has hundreds of rows at most). An Approval, stop or cancel written by another process is seen on the next tick with no cache to invalidate.

## The scheduler is a pure function

Inside `drive`, deciding what to do next is separated from doing it:

```rust
pub(crate) enum Action {
    StartIteration { loop_: StepRunId },
    ExpandItems { step: StepRunId },
    Launch { step: StepRunId },
    RunOperation { step: StepRunId, op: OperationId },
    Block { step: StepRunId, reason: BlockReason },
    FinishRun { run: RunId, state: RunState },
    Return(DriveOutcome),
    Wait,
}

pub(crate) struct Ctx { limits: FrozenLimits, free_slots: usize, paused_until: Option<Timestamp>, now: Timestamp, stop: StopLevel }

pub(crate) fn decide(graph: &RunGraph, ctx: &Ctx) -> Vec<Action>;
```

The hard rules of [G. State Machines](G-state-machines.md) (Attempt and Iteration limits, Transient Failures not spending an Attempt, dependents of a blocked Task waiting, a run ending only `succeeded` or `cancelled`) are table tests on `decide`: build a `RunGraph`, check the `Action`s. This is an internal seam of `drive`; it does not appear in the facade.

`RunGraph`, `Action`, `Ctx` and the state enums are defined beside `decide` and import neither `sqlx` nor the `git` module. `store` depends on them and maps rows into a `RunGraph`; the dependency never points the other way. This is what keeps the domain free of the database while `drive` itself calls the concrete `Store` ([ADR 0012](../adr/0012-agent-runtime-is-the-only-trait.md) records that exception).

## Errors and outcomes

```rust
pub enum DriveOutcome {
    AtGate { gate: GateRef },              // exit 0
    Succeeded { merge: MergeResult },      // exit 0, also when "not merged: base moved"
    Blocked { steps: Vec<BlockedStep> },   // exit 4, also when other Tasks merged
    Cancelled,                             // exit 5
    Interrupted,                           // exit 130
    Halted(Refusal),                       // exit 3, after running work finished (pinned Revision edited)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("refused: {0:?}")]
    Refused(Refusal),                      // exit 3, nothing changed
    #[error(transparent)]
    Internal(#[from] InternalError),       // exit 1
}

pub enum Refusal {
    DriverLive { pid: u32, since: Timestamp },
    ActiveRunExists { run: RunId },
    AmbiguousSubject { candidates: Vec<SubjectRef> },
    UnknownSubject { query: String },
    GateNotWaiting { .. },
    StaleRevision { expected: RevisionRef, current: RevisionRef },
    PinnedRevisionEdited { .. },
    RunBranchMoved { expected: Sha, found: Sha },
    UnrecordedEdit { .. },
    SchemaTooNew { db: i64, supported: i64 },
    GitTooOld { found: Version, min: Version },
    NoDriverToStop,
    // …
}
```

- **Refusals are data.** A `Refusal` carries observed values and no text. The CLI owns the `✖ … / fix: …` wording of [J](J-cli.md#error-messages), because the ways out are CLI commands with full names.
- **Blocked, cancelled and interrupted are outcomes, not errors.** They are what `drive` reports when it returns normally.
- `surt-engine` uses `thiserror`. `surt` uses `anyhow` only on the path to `✖ internal error` (exit `1`). clap keeps exit `2`.

## Progress and stopping

The engine never prints. It reports typed events, and the CLI renders lines, the TTY footer, the five-minute line without a TTY and the ASCII fallback ([J](J-cli.md#foreground-progress)). Surt's own diagnostics go through `tracing`, shown only with `SURT_LOG`.

```rust
pub enum Progress {
    StepStarted { .. }, StepFinished { .. }, AttemptRetry { .. }, Merged { key: ItemKey, sha: Sha },
    Salvaged { .. }, OrphanStopped { .. }, Paused { until: Option<Timestamp> },
    Activity(ActivitySnapshot),       // running steps + start times, queued, slots: input for the footer
    Cleanup { .. },
}
pub type ProgressSink = tokio::sync::mpsc::UnboundedSender<Progress>;

pub struct StopSignal(/* tokio::sync::watch<StopLevel> */);
pub enum StopLevel { None, Graceful, Force }
```

`drive` emits `Activity` when it changes, on the ticks where the Driver waits for running work, so a snapshot never shows a step it has just decided to launch as queued. The footer and the five-minute line are pure functions of (last snapshot, time of the last event line, `now`, whether stderr is a TTY) in the CLI's progress module; a one-second Tokio interval only calls them.

The CLI's first Ctrl-C (or SIGHUP, or the console closing) calls `graceful()`, the second `force()`. A stop request written by `surt stop` is read by `drive` on its next tick and raised on the same signal, so both paths run the same Interruption ([A. Process Model](A-process-model.md#interruption-ctrl-c-and-cancellation)).

## Built-in definitions: the `Catalog`

```rust
// crates/surt/src/builtin.rs
pub fn catalog() -> Catalog {
    Catalog::new()
        .workflow("feature", include_str!("../assets/workflows/feature.toml"))
        .workflow("task-cycle", include_str!("../assets/workflows/task-cycle.toml"))
        .agent("builder", AgentAssets {
            toml: include_str!("../assets/agents/builder/agent.toml"),
            instructions: include_str!("../assets/agents/builder/instructions.md"),
            schemas: &[("schemas/commit.schema.json", include_str!("../assets/agents/builder/schemas/commit.schema.json"))],
        })
        // spec, tasks, reviewer, shipper
}
```

- The table is explicit, with `include_str!`; no directory-embedding crate.
- The engine merges `.surt/agents/<name>/` over the `Catalog`, validates and freezes. It does not know where the bytes came from.
- The engine embeds only what it owns: its migrations and the schemas of the Artifacts that Operations produce (`base`, `check-report`, `conflict`, `integration`). Agent schemas (`manifest`, `commit`, `feedback`, `delivery`) travel with their Agents in `surt`.

## Toolchain and dependencies

`edition = "2024"`, `rust-version = "1.94"` (sqlx 0.9's floor; `File::lock` needs 1.89), a pinned stable in `rust-toolchain.toml`, Tokio multi-threaded.

| Crate | In | For |
|---|---|---|
| `tokio` (rt-multi-thread, macros, process, signal, sync, time, fs) | both | async, child processes, Ctrl-C/SIGHUP/console close |
| `sqlx` 0.9 (`sqlite`, `runtime-tokio`, `migrate`; no defaults) | engine | [D](D-database.md) |
| `serde`, `serde_json`, `toml` | engine | definitions, deliveries, configuration |
| `jsonschema` (no defaults, no HTTP resolver) | engine | validating delivered files, including schemas overridden in `.surt/agents/` |
| `uuid` (`v7`), `sha2` | engine | ids, blobs, definition identity |
| `jiff` | engine | Unix-second `Timestamp`s, their display and the local date of `YYYYMMDD-<slug>` |
| `process-wrap` 10 (tokio, process-group, job-object) | engine | process trees ([A](A-process-model.md#supervising-agent-processes)) |
| `libc` (unix), `windows-sys` (windows) | engine | process start time and boot id, `CTRL_BREAK_EVENT` |
| `dunce` | engine | canonical paths without `\\?\` |
| `thiserror` 2, `tracing` | engine | errors, diagnostics |
| `clap` 4 (derive), `anyhow`, `tracing-subscriber` (env-filter), `anstream`/`anstyle` | surt | the CLI, `SURT_LOG`, colour and `NO_COLOR` |
| `tempfile` | dev | tests |

Not used: `dirs`/`directories` (`std::env::home_dir` is correct on Windows and no longer deprecated), `indicatif`/`crossterm` (the footer is `\r` plus ANSI, which current Windows consoles accept), `async-trait`, `git2`/`gix` ([A. Git Integration](A-git-integration.md)), `chrono`/`time`, `include_dir`.

## Rule exceptions

Deviations of the Milestone 1 code from [`rules/`](../../rules) and from this document that are kept on purpose, in the rules' report format. [D](D-database.md#rule-exceptions) and [ADR 0012](../adr/0012-agent-runtime-is-the-only-trait.md) record the database and DIP ones.

```
Exceptions: rust.md unsafe; surt-engine platform/process/{macos,windows,signals}.rs, platform/children/{unix,windows}.rs; process start time, boot id, signals, PR_SET_PDEATHSIG and console control events have no safe std API; every unsafe block is one libc/windows-sys call inside platform, which is the only module allowed to touch the OS
Exceptions: B toolchain table (libc, windows-sys only); surt-engine Cargo.toml windows, platform/children/windows.rs:own_tree; process-wrap's CreationFlags takes windows::Win32::System::Threading::PROCESS_CREATION_FLAGS, and the windows crate is already in the build through process-wrap; it only names that type, every call goes through windows-sys
Exceptions: spec "every OS #[cfg] lives in platform"; surt driving/interruption.rs:Interruptions; the rule governs the engine's module split, and tokio::signal exposes Ctrl-C/SIGHUP and Ctrl-C/console-close as different types per OS; the CLI keeps two cfg blocks instead of the engine owning console signals
Exceptions: spec "every OS #[cfg] lives in platform"; #[cfg(unix)] on tests in crates/surt/tests/cli and src/**/tests; those tests send POSIX signals or run sh, which only exist on Unix; Windows CI does not run them
Exceptions: C rule 6 / B "the core never names Task"; drive/worktrees.rs branch surt/task/<subject>/<key>, drive/integrate.rs "Merge task" message and Surt-Task trailer, drive/snapshot.rs integration "tasks" key; A. Git Integration and schemas/integration.schema.json fix these formats for the engine's own Operations; making them workflow-provided adds configuration no second workflow needs yet
Exceptions: architecture.md avoid cycles; project::{error, layout, config} imported by drive, reconcile, cleanup, artifacts; the spec fixes the twelve engine modules and lists Error and Refusal as facade types of project; those submodules import nothing that imports them back, so files have no cycle, but the module-level edges project -> drive -> project remain, as do drive <-> reconcile (Progress) and store -> drive::graph (sanctioned above)
Exceptions: clean-code.md primitive obsession; store/*.rs methods taking &str ids, ref_move(step), revision(id); runtime/error.rs:RuntimeError.code; rows carry the same TEXT ids the database holds and only the Driver and cancel paths hold a RunId (those methods take &RunId); RuntimeError.code relays Claude Code's open-ended error categories verbatim into execution.error_code, while the decision is the closed FailureClass
Exceptions: rust.md typed errors; project/error.rs:InternalError(Box<dyn Error>); an internal error (exit 1) is reported, never matched, so one boxed type keeps StoreError, GitError, io and snapshot corruption out of the public API
Exceptions: B facade method list; project:{open_read_only, resolve_subject, subject_named, reference, resolve_execution, inspect_execution, gate_status, blocked_steps, integration_branch, cancel_impact, wait_for_driver, artifacts, free_subject_name, active_subject_with_slug, resolve_commit, actor}; the CLI needs them for J's read-only commands, next-command hints, naming, cancel confirmation and `surt stop`'s wait; each is a read the facade's documented methods would otherwise duplicate
Exceptions: D budget_base; store/control.rs:unblock; it stores the number of Executions of any state (or Iterations) when the step last became running, and decide counts failed non-transient Attempts after that index; same budget as D, different base value, so the column is not comparable with D's examples
Exceptions: A agent slots as lock files; drive/flights.rs:Flights; Milestone 1 counts slots inside one Driver, so two Drivers of one project can each use max_concurrent_agents; status shows the per-Driver count
Exceptions: testing.md no timing in tests; platform/tests.rs, platform/process/tests.rs, runtime/claude_code/tests/stub.rs, project/tests/drive/recovery.rs, crates/surt/tests/cli/interrupt.rs; real processes and OS locks have no virtual clock, and tokio::time::pause() breaks sqlx's pool acquire (PoolTimedOut); every wait is bounded polling with a generous ceiling, never a fixed sleep that a result depends on
Exceptions: A orphan stop; platform/process/windows.rs:signal; on Windows a stale agent is stopped with TerminateProcess on its root pid only; the Job Object with kill-on-close already ends the tree when the Driver that owned it dies, so only a tree whose Job outlived its Driver can leave grandchildren
Exceptions: Linux PR_SET_PDEATHSIG; platform/children/unix.rs:end_with_surt; the signal fires when the spawning thread exits, not the process; children are spawned from Tokio worker threads, which live until the runtime stops at exit, and never from spawn_blocking threads, which can idle out
Exceptions: clean-code.md 200 lines/file; store/approvals.rs, store.rs, store/executions.rs, store/graph.rs, store/reading.rs, drive/launch.rs, drive/driver.rs, drive/load.rs, drive/frozen.rs, drive/integrate.rs, git.rs, artifacts/working_copy.rs, project/status.rs, project/gates.rs, project/error.rs, runtime/fake.rs, runtime/claude_code/invocation.rs, surt refusal.rs (201-303 lines); each holds one checkpoint family, one Operation or one enum and its rendering; splitting would separate a transaction from its row types or a match from its arms
Exceptions: clean-code.md 20 lines/function (over 60); surt refusal.rs:message, driving/progress.rs:message, refusal/definition.rs:describe; one arm per Refusal, Progress or Problem variant, exhaustive on purpose so a new variant cannot ship without wording
Exceptions: clean-code.md 20 lines/function (over 60); store/approvals.rs:record_decision, store/executions.rs:start_execution, store/acceptance.rs:accept_execution; each is one BEGIN IMMEDIATE checkpoint that writes its rows and its event together, as this document requires
```

## Decided elsewhere

- `drive`, locks, slots and process supervision: [A. Process Model](A-process-model.md).
- The Operation registry and Git commands: [A. Git Integration](A-git-integration.md).
- The `AgentRuntime` protocol and acceptance rules: [F. Agent Contract](F-agent-contract.md#agent-runtime-and-fake-runtime).
- Checkpoints the store methods implement: [I. Recovery](I-recovery.md#transactional-checkpoints).
- Commands, messages and exit codes: [J. CLI Surface](J-cli.md).
- How tests drive the Fake Runtime and which checkpoints are covered: [Test strategy](../../.scratch/surt-architecture/issues/16-test-strategy.md).
