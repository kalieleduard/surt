# Rust workspace structure and core traits

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Assignee: Kaliel
Blocked by: 05, 06, 07, 11

## Question

How is the code organized into crates and modules, and which traits exist between them?

Decide, for v1 only:
- **Crates:** a single one or `surt-core` + `surt-cli`? Justify it by the real boundary (the CLI is the only one that knows the phases of the `feature` workflow).
- **Core modules:** engine, store, runtime, git/operations, artifacts, recovery, cleanup — what each one hides.
- **Traits** only where there are two real implementations: `AgentRuntime` (Claude Code + Fake), and maybe `Clock`. `StateStore`/`GitOps` only if tests really need to swap them.
- **Errors:** `thiserror` in the core × `anyhow` in the CLI, and how an error carries the failure class (`transient` / `not_retryable` / `retryable`) and becomes an exit code.
- **Dependencies:** short list of what goes into M1.
- **Built-ins:** the `feature`/`task-cycle` workflows and agents via `include_str!`.

Output: sections A (Mermaid diagram) and B. Use `/codebase-design`.

## Context

- **AgentRuntime:** sketch `execute(request, events, stop)` in [F](../../../docs/architecture/F-agent-contract.md#agent-runtime-and-fake-runtime); the Fake Runtime goes through the same validation and does not write to SQLite.
- **Workflow:** the parser/validator belongs to the core — five `kind`s, `deny_unknown_fields`, static checks that read `agent.toml` and the Operations registry ([E](../../../docs/architecture/E-workflow-schema.md)).
- **Operations:** registry id → Inputs, Outcomes, artifacts; the six Git ones talk only to the `git` CLI (argv, no shell) ([A](../../../docs/architecture/A-git-integration.md)).
- **Store:** `sqlx::query_as` + `FromRow` at runtime, `sqlx::migrate!` under `db.lock`, writes always `BEGIN IMMEDIATE`, every state writes its `event` in the same transaction; reference migration [`0001_initial.sql`](../../../docs/architecture/migrations/0001_initial.sql) ([D](../../../docs/architecture/D-database.md)).
- **CLI:** only it knows the phases, the `plan`/`start` preconditions and the gate → next command table; reads take no lock and do not reconcile; exit codes 0/1/2/3/4/5/130 ([J](../../../docs/architecture/J-cli.md)).
- **Cleanup:** idempotent steps after the terminal state, under the run's lock; `surt delete` = Git/disk and then the purge in one transaction ([I. Cleanup](../../../docs/architecture/I-cleanup.md)). Lives in the core (generic: worktrees, refs, directories), not in the CLI.

## Answer

Resolved by human exchange in two rounds (12 questions, all accepted with the recommendation). Full section in [B. Rust Workspace, Modules and Traits](../../../docs/architecture/B-workspace.md); component diagram and the flow of a command in [A. Overview](../../../docs/architecture/A-overview.md). D and F now point to B. New ADRs: [0011 — Two crates keep the engine generic](../../../docs/adr/0011-two-crates-keep-the-engine-generic.md) and [0012 — The Agent Runtime is the only trait](../../../docs/adr/0012-agent-runtime-is-the-only-trait.md).

Final decisions:

- **Crates:** a workspace with `surt-engine` (generic lib) and `surt` (bin: clap, phases, texts, exit codes, built-in workflows and agents). The compiler prevents the engine from importing the CLI; engine tests use synthetic workflows because the real ones do not exist in the crate.
- **Facade:** the CLI only sees `Project` (`open`, `find_subject`, `create_run`, `drive`, `approve`, `reject`, `request_stop`, `cancel`, `restore`, `delete`, `status`, `inspect`, `show`). `drive` = `try_lock` → reconcile → advance → cleanup. Phase preconditions and confirmations stay in the CLI; Subject resolution (name/slug/prefix) in the engine.
- **Engine modules:** `project`, `workflow`, `drive`, `reconcile`, `artifacts`, `context`, `runtime`, `operations`, `git`, `cleanup`, `store`, `platform`; only `project` and the facade types are public.
- **Operations:** a closed registry (`enum` + `match`) in the engine, no trait. **Run Branch** now means the branch of any Workflow Run (root or child), aligned with the `run_branch` table; Integration Branch and Task Branch are the `feature` workflow's names. Renamed ids: `git.create_run_branch`, `git.create_child_worktree`, `git.snapshot_run_branch` (A-git, E, D, I, `integration.schema.json`; closed tickets were not rewritten).
- **Traits:** only `AgentRuntime` (Claude Code + Fake), native `async fn` with a `Send` future, dispatch through `&impl AgentRuntime` in `drive`. No trait for store, Git, Operations or clock: real SQLite and `git` in a tempdir, `tokio::time::pause()`, computations taking `now` as an argument.
- **Store:** one method per transactional checkpoint of I (each one a `BEGIN IMMEDIATE` with its `event`), no transaction exposed; `drive` rereads the `RunGraph` on every tick.
- **Scheduler:** a pure function `decide(&RunGraph, &Ctx) -> Vec<Action>`; the rules of G become table tests without I/O.
- **Errors:** `thiserror` in the engine; `Error = Refused(Refusal) | Internal`; `Refusal` is data without text (the CLI writes `✖ … / fix: …`); `DriveOutcome = AtGate | Succeeded | Blocked | Cancelled | Interrupted | Halted(Refusal)` maps to 0/0/4/5/130/3. `FailureClass` lives in the adapter's `RuntimeError` and is recorded on the Execution, never crossing the facade. `anyhow` only in the CLI.
- **Progress and stop:** the engine never prints; it emits typed `Progress` over `mpsc`. Ctrl-C (graceful/force) and the `surt stop` request in SQLite arrive through the same `StopSignal` (`watch<StopLevel>`).
- **Built-ins:** a `Catalog` assembled in the `surt` crate with `include_str!`, passed to `Project::open`; the engine merges `.surt/agents/`. The engine embeds only migrations and the artifact schemas of Operations (`base`, `check-report`, `conflict`, `integration`).
- **Toolchain and dependencies:** edition 2024, `rust-version = "1.94"`, pinned stable, multi-thread Tokio; `sqlx` 0.9, `serde`/`serde_json`/`toml`, `jsonschema` (no HTTP), `uuid` v7, `sha2`, `jiff`, `process-wrap` 10, `libc`/`windows-sys`, `dunce`, `thiserror`, `tracing`; in the CLI `clap`, `anyhow`, `tracing-subscriber`, `anstream`/`anstyle`; `tempfile` in dev. Out: `dirs` (verified fact: `std::env::home_dir` not deprecated in rustc 1.99), `indicatif`/`crossterm`, `async-trait`, `git2`/`gix`, `chrono`/`time`, `include_dir`.

`CONTEXT.md`: new term **Run Branch**; Integration Branch and Task Branch redefined as Run Branches of the `feature` workflow. Context notes added to [Test strategy](16-test-strategy.md) and [Roadmap and Milestone 1](17-roadmap-and-milestone-1.md). No new ticket.

Review against `rules/` (2026-10-17): `drive` depending on concrete `Store` and `git` goes against the DIP in `rules/architecture.md`; kept as an exception recorded in [ADR 0012](../../../docs/adr/0012-agent-runtime-is-the-only-trait.md), on the condition that `RunGraph`, `Action`, `Ctx` and the state enums do not import `sqlx` or `git` and that `store` maps rows to them ([B](../../../docs/architecture/B-workspace.md#the-scheduler-is-a-pure-function)). `jiff` now serves timestamps in Unix seconds.
