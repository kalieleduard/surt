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
└── surt/                     # bin "surt": the CLI and the feature Template
    ├── assets/templates/feature/workflows/{feature,task-cycle}.toml
    ├── assets/templates/feature/agents/<agent>/{agent.toml,instructions.md,schemas/…}
    └── src/…
```

- **`surt-engine`** knows the engine layer of [C. Domain Model](C-domain-model.md#two-layers) and nothing else. It cannot import anything from `surt`, so the compiler enforces that the core never names Spec, Task, Builder, Reviewer, Shipper or a phase.
- **`surt`** owns everything that is about a terminal: clap, `--input` parsing, the next-command hints (always `surt continue <subject>`), every message text, exit codes, and the bytes of the feature **Template** (`feature`/`task-cycle` and their five Agents), which `surt init --template feature` writes into `.surt/`. Apart from those bytes it knows no workflow: `surt run` and `surt continue` take any workflow under `.surt/workflows/`.
- The feature workflow's files do not exist inside `surt-engine`. Engine tests therefore write their own synthetic workflows and Agents into the test project's `.surt/`, which keeps the core honest about being generic.

Rationale and rejected alternatives: [ADR 0011](../adr/0011-two-crates-keep-the-engine-generic.md).

## The engine's interface: `Project`

The CLI sees one facade type and the types it returns. Everything else in `surt-engine` is `pub(crate)`.

```rust
pub struct Project { /* .surt/ root, store, config */ }

impl Project {
    /// Creates `.surt/` (gitignore, config, database) and writes the Template's
    /// missing files, given as (path under `.surt/`, bytes). Never overwrites;
    /// reports each file as created or kept. Knows no Template name.
    pub async fn init(cwd: &Path, template: &[(&str, &[u8])]) -> Result<Initialized, Error>;

    /// Finds `.surt/` from `cwd`, opens the database (migrations under `db.lock`),
    /// loads project + global configuration. Refuses a database newer than the binary.
    pub async fn open(cwd: &Path) -> Result<Project, Error>;

    /// Full name, slug or unique prefix. `Refusal::AmbiguousSubject` / `UnknownSubject`.
    pub async fn find_subject(&self, query: &str) -> Result<SubjectRef, Error>;

    /// Reads the closure of definitions and agents from `.surt/workflows/` and
    /// `.surt/agents/`, validates and freezes it, writes the "run created"
    /// checkpoint. Refuses when the Subject already has an active run.
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
- **There are no phases.** `surt continue` calls `drive` on the active run whatever step it is at, with no gate precondition in the CLI; the engine only sees Approval Gates declared in the frozen definition, and a gate that still waits ends the drive at once.
- **Confirmations are not in the engine.** `cancel`, `restore` and `delete` act when called; the CLI asks first, using `status`/`inspect` to show the impact.
- **Subject resolution is in the engine.** A Subject is an engine concept; how a name, slug or prefix matches is the same for every workflow.

## Modules of `surt-engine`

| Module | Hides | Dependency category |
|---|---|---|
| `project` | the facade; finding `.surt/`; project and global configuration; writing a Template's files at `init`; reading a closure's definition files from `.surt/workflows/` and `.surt/agents/` (the adapter behind `workflow`'s definition source) | — |
| `workflow` | parsing the five `kind`s with `deny_unknown_fields`; static validation against agents and the Operation registry; resolving the closure of a root workflow through a definition source, which only `Project::create_run` backs with the files under `.surt/`; freezing the closure and its SHA-256 ([E](E-workflow-schema.md), [F](F-agent-contract.md#definitions-in-the-project)) | in-process, pure |
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

`project` is the only public module, plus the types the facade exposes: `runtime::{AgentRuntime, ClaudeCodeRuntime, FakeRuntime}` and their request/response types, `Progress`, `StopSignal`, `DriveOutcome`, `Error`, `Refusal` and the read views. `project::check_workflow(root, workflow, initial_artifacts)` runs the same static validation on the files under `<root>/.surt/` without opening a database; the binary's Template tests use it.

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
    Launch { step: StepRunId, slot: usize },
    RunOperation { step: StepRunId, op: OperationId },
    Block { step: StepRunId, reason: BlockReason },
    FinishRun { run: RunId, state: RunState },
    Return(DriveOutcome),
    Wait,
}

pub(crate) struct Ctx {
    limits: Limits,             // max_attempts, max_iterations, frozen at root run creation
    free_slots: Vec<usize>,     // slot indices this Driver locked this tick (A), one per Launch
    stop: StopLevel,
    paused_until: Option<i64>,  // Unix seconds; no agent Launch while now < paused_until
    now: i64,
}

pub(crate) struct Decision { actions: Vec<Action>, queued: usize } // queued: ready agents left without a slot or paused
pub(crate) fn decision(graph: &RunGraph, ctx: &Ctx) -> Decision;
```

`paused_until` comes from the pause schedule, a pure function of the pause state, the failure just observed and `now` (`drive/pause.rs`; [I](I-recovery.md#transient-failures-network-and-rate-limits)), so neither it nor `decide` reads a clock. While the pause holds, `decide` still emits `RunOperation`: Checks and integrations do not wait.

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
    InvalidDefinition(Box<InvalidDefinition>), // file under .surt/, field or step, Problem
    UnknownWorkflow { name: String, known: Vec<String> },
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
    Salvaged { .. }, OrphanStopped { .. },
    Paused { pause: ActivityPause, waiting: usize }, Resumed { paused_seconds: i64 }, // ActivityPause { until: i64, backoff: Option<i64> }
    ConfigInvalid { path: PathBuf, line: Option<usize>, reason: String, limit: usize }, // live limit kept (A)
    Activity(ActivitySnapshot),       // running steps + start times, queued, project-wide slots, the pause: input for the footer
    Cleanup { .. },
}
pub type ProgressSink = tokio::sync::mpsc::UnboundedSender<Progress>;

pub struct StopSignal(/* tokio::sync::watch<StopLevel> */);
pub enum StopLevel { None, Graceful, Force }
```

`drive` emits `Activity` when it changes, on the ticks where the Driver waits for running work, so a snapshot never shows a step it has just decided to launch as queued. The footer and the five-minute line are pure functions of (last snapshot, time of the last event line, `now`, whether stderr is a TTY) in the CLI's progress module; a one-second Tokio interval only calls them.

The CLI's first Ctrl-C (or SIGHUP, or the console closing) calls `graceful()`, the second `force()`. A stop request written by `surt stop` is read by `drive` on its next tick and raised on the same signal, so both paths run the same Interruption ([A. Process Model](A-process-model.md#interruption-ctrl-c-and-cancellation)).

## Definitions in the project, the Template in the binary

```rust
// crates/surt/src/template/feature.rs
const BUILDER: AgentFiles = &[
    ("agent.toml", include_str!("../../assets/templates/feature/agents/builder/agent.toml")),
    ("instructions.md", include_str!("../../assets/templates/feature/agents/builder/instructions.md")),
    COMMIT_SCHEMA, FEEDBACK_SCHEMA, CHECK_REPORT_SCHEMA, CONFLICT_SCHEMA,
];
pub(super) const TEMPLATE: Template = Template {
    workflows: &[("feature.toml", include_str!(…)), ("task-cycle.toml", include_str!(…))],
    agents: &[("spec", SPEC), ("tasks", TASKS), ("builder", BUILDER), ("reviewer", REVIEWER), ("shipper", SHIPPER)],
};
```

- The table is explicit, with `include_str!`; no directory-embedding crate. `surt init --template feature` flattens it into (path under `.surt/`, bytes) pairs for `Project::init`, which writes the missing ones. The engine knows no Template name.
- An Agent that reads an Operation's artifact (`check-report`, `conflict`, `integration`, `base`) references a copy in its own `schemas/`. The Template writes those copies from the engine's exported constants (`CHECK_REPORT_SCHEMA`, `CONFLICT_SCHEMA`, `INTEGRATION_SCHEMA`, `BASE_SCHEMA`).
- `Project::create_run` reads exactly the closure of the new root run from `.surt/`: the named workflow, its `per_item` children transitively and the Agents they reference. Nothing else reads `.surt/workflows/` or `.surt/agents/`, so a broken file outside the closure never refuses. There is no compiled `Catalog`, no merge over defaults and no fallback ([ADR 0013](../adr/0013-definitions-live-in-the-project.md)). An invalid file refuses with `InvalidDefinition`, a missing root workflow with `UnknownWorkflow`.
- The engine embeds only what it owns: its migrations, the `.gitignore` and `config.toml` that `init` writes, and the schemas of the Artifacts that Operations produce (`base`, `check-report`, `conflict`, `integration`), against which it validates its Operations' outputs. Agent schemas (`manifest`, `commit`, `feedback`, `delivery`) are Template files.

## Toolchain and dependencies

`edition = "2024"`, `rust-version = "1.94"` (sqlx 0.9's floor; `File::lock` needs 1.89), a pinned stable in `rust-toolchain.toml`, Tokio multi-threaded.

| Crate | In | For |
|---|---|---|
| `tokio` (rt-multi-thread, macros, process, signal, sync, time, fs) | both | async, child processes, Ctrl-C/SIGHUP/console close |
| `sqlx` 0.9 (`sqlite`, `runtime-tokio`, `migrate`; no defaults) | engine | [D](D-database.md) |
| `serde`, `serde_json`, `toml` | engine; `toml` also in surt | definitions, deliveries, configuration; `surt answer --file` |
| `jsonschema` (no defaults, no HTTP resolver) | engine | validating delivered files against the schemas read from `.surt/agents/` |
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
Exceptions: rust.md unsafe; surt-engine platform/process/{macos,windows,signals}.rs, platform/children/{unix,windows}.rs, platform/console/{unix,windows}.rs; process start time, boot id, signals, PR_SET_PDEATHSIG, console control events, the console code page and the terminal width have no safe std API; every unsafe block is one libc/windows-sys call inside platform, which is the only module allowed to touch the OS
Exceptions: B toolchain table (libc, windows-sys only); surt-engine Cargo.toml windows, platform/children/windows.rs:own_tree; process-wrap's CreationFlags takes windows::Win32::System::Threading::PROCESS_CREATION_FLAGS, and the windows crate is already in the build through process-wrap; it only names that type, every call goes through windows-sys
Exceptions: spec "every OS #[cfg] lives in platform"; surt driving/interruption.rs:Interruptions; the rule governs the engine's module split, and tokio::signal exposes Ctrl-C/SIGHUP and Ctrl-C/console-close as different types per OS; the CLI keeps two cfg blocks instead of the engine owning console signals
Exceptions: spec "every OS #[cfg] lives in platform"; #[cfg(unix)] on tests in crates/surt/tests/cli and src/**/tests; those tests send POSIX signals or run sh, which only exist on Unix; Windows CI does not run them
Exceptions: architecture.md avoid cycles; project::{error, layout, config} imported by drive, reconcile, cleanup, artifacts; the spec fixes the twelve engine modules and lists Error and Refusal as facade types of project; those submodules import nothing that imports them back, so files have no cycle, but the module-level edges project -> drive -> project remain, as do drive <-> reconcile (Progress) and store -> drive::graph (sanctioned above)
Exceptions: clean-code.md primitive obsession; store/*.rs methods taking &str ids, ref_move(step), revision(id); runtime/error.rs:RuntimeError.code; rows carry the same TEXT ids the database holds and only the Driver and cancel paths hold a RunId (those methods take &RunId); RuntimeError.code relays Claude Code's open-ended error categories verbatim into execution.error_code, while the decision is the closed FailureClass
Exceptions: rust.md typed errors; project/error.rs:InternalError(Box<dyn Error>); an internal error (exit 1) is reported, never matched, so one boxed type keeps StoreError, GitError, io and snapshot corruption out of the public API
Exceptions: B facade method list; project:{open_read_only, resolve_subject, subject_named, reference, resolve_execution, inspect_execution, logs, execution_logs, gate_status, blocked_steps, integration_branch, cancel_impact, restore_impact, delete_impact, wait_for_driver, artifacts, free_subject_name, active_subject_with_slug, resolve_commit, actor, base_merge, interrupted_merge}; the CLI needs them for J's read-only commands, next-command hints, naming, the cancel, restore and delete confirmations, `surt stop`'s wait, the merge line after `✔ delivered` and the `merge_into_base_interrupted` block's how-to; each is a read the facade's documented methods would otherwise duplicate
Exceptions: D budget_base; store/control.rs:unblock; it stores the number of Executions of any state (or Iterations) when the step last became running, and decide counts failed non-transient Attempts after that index; same budget as D, different base value, so the column is not comparable with D's examples
Exceptions: testing.md no timing in tests; platform/tests.rs, platform/process/tests.rs, runtime/claude_code/tests/stub.rs, project/tests/drive/recovery.rs, crates/surt/tests/cli/interrupt.rs, crates/surt/tests/cli/follow.rs, crates/surt/tests/cli/slots.rs; real processes and OS locks have no virtual clock, and tokio::time::pause() breaks sqlx's pool acquire (PoolTimedOut); every wait is bounded polling with a generous ceiling, never a fixed sleep that a result depends on
Exceptions: A orphan stop; platform/process/windows.rs:signal; on Windows a stale agent is stopped with TerminateProcess on its root pid only; the Job Object with kill-on-close already ends the tree when the Driver that owned it dies, so only a tree whose Job outlived its Driver can leave grandchildren
Exceptions: Linux PR_SET_PDEATHSIG; platform/children/unix.rs:end_with_surt; the signal fires when the spawning thread exits, not the process; children are spawned from Tokio worker threads, which live until the runtime stops at exit, and never from spawn_blocking threads, which can idle out
Exceptions: clean-code.md 200 lines/file; store/approvals.rs, store.rs, store/executions.rs, store/graph.rs, store/reading.rs, drive/launch.rs, drive/driver.rs, drive/load.rs, drive/frozen.rs, drive/integrate.rs, git.rs, artifacts/working_copy.rs, project/status.rs, project/gates.rs, project/error.rs, runtime/fake.rs, surt refusal.rs (212-323 lines); each holds one checkpoint family, one Operation or one enum and its rendering; splitting would separate a transaction from its row types or a match from its arms
Exceptions: clean-code.md 200 lines/file; project.rs (203 lines); it is the facade: the module list, the public re-exports and the Project struct with its opening; the facade's methods live in the submodules, so the only way down is to scatter the re-exports, which is what this file exists to gather
Exceptions: clean-code.md primitive obsession; drive/base_merge.rs:BaseMerge::Merged{from, to}, project/interrupted_merge.rs:InterruptedMerge{from, to}; full SHAs as String like every other SHA the store, git and the JSON documents carry (RefMove, Commit, run_branch rows); a Sha newtype would have to start at the store and git boundary to mean anything
Exceptions: clean-code.md 20 lines/function (over 60); surt refusal.rs:message, driving/progress/wording.rs:message, refusal/definition.rs:describe; one arm per Refusal, Progress or Problem variant, exhaustive on purpose so a new variant cannot ship without wording
Exceptions: clean-code.md 20 lines/function (over 60); store/approvals.rs:record_decision, store/executions.rs:start_execution, store/acceptance.rs:accept_execution; each is one BEGIN IMMEDIATE checkpoint that writes its rows and its event together, as this document requires
```

## Decided elsewhere

- `drive`, locks, slots and process supervision: [A. Process Model](A-process-model.md).
- The Operation registry and Git commands: [A. Git Integration](A-git-integration.md).
- The `AgentRuntime` protocol and acceptance rules: [F. Agent Contract](F-agent-contract.md#agent-runtime-and-fake-runtime).
- Checkpoints the store methods implement: [I. Recovery](I-recovery.md#transactional-checkpoints).
- Commands, messages and exit codes: [J. CLI Surface](J-cli.md).
- How tests drive the Fake Runtime and which checkpoints are covered: [Test strategy](../../.scratch/surt-architecture/issues/16-test-strategy.md).
