# K. Roadmap and Milestone 1

What gets built first, in which order, and what follows. Sections A–J describe **v1**, the whole design. Milestone 1 is the smallest subset of v1 that delivers a Feature end to end. Every cut below is a subset of v1, not a different design, and none needs a migration. Vocabulary is defined in [`CONTEXT.md`](../../CONTEXT.md). Decision ticket: [Roadmap + Milestone 1 proposal](../../.scratch/surt-architecture/issues/17-roadmap-and-milestone-1.md).

## Milestone 1

**Goal:** a small Feature goes from `surt feature` to a delivered Integration Branch with the real Claude Code, and survives a `kill -9` at any point.

**Done when:**

1. `cargo fmt --all -- --check`, `cargo clippy --all-targets --features test-support -- -D warnings` and `cargo test --features test-support` pass in CI on macOS, Linux and Windows ([`rules/rust.md`](../../rules/rust.md)). The suite holds everything in [Test strategy](../../.scratch/surt-architecture/issues/16-test-strategy.md): the brief's E2E with the Fake Runtime (2+ Tasks, a rejected review, a new Builder, the Shipper, cleanup), fault points at the eight checkpoints of [I. Recovery](I-recovery.md#transactional-checkpoints), the `kill` smoke, and the three Git scenarios. The E2E stays under 60 s per OS.
2. A manual smoke with the real Claude Code passes **on macOS**: in a toy repository, a Feature with two Tasks reaches `succeeded`. Its script is the checklist `docs/smoke-claude-code.md`.

### What the user sees

```
$ surt init
$ surt feature "add login"             # Spec Agent → stops at the spec gate
$ $EDITOR .surt/features/20261017-add-login/spec.md     # optional
$ surt approve add-login
$ surt plan add-login                  # Task Agent → manifest + tasks/*.md → gate
$ surt approve add-login
$ surt start add-login                 # builders, checks, review, integrate, shipper, cleanup
✔ task A merged (3c1aa20)
↻ task B build attempt 2
✔ task B merged (9e04b7d)
✔ shipper    delivered — surt/20261017-add-login @ 4e1b2c0
✔ run        succeeded
↻ cleanup    2 worktrees · 2 task branches · working copy
```

### Commands

Ten of the fourteen commands of [J](J-cli.md#commands). A command or flag that is cut does not exist in clap (exit `2`). Exit codes and the message shape are those of J.

| In Milestone 1 | Cut until |
|---|---|
| `init` | — |
| `feature "<description>"` with `--name`, `--base`, `--new` | `--merge`/`--no-merge`: M4 |
| `feature <feature>` (resume the spec phase) | `--new-run`: M2 |
| `plan`, `start`, `approve [--rev N]`, `reject -m [--rev N]` | — |
| `cancel [--yes]` | — |
| `status [<feature>] [--all]` | `--run`, `--json`: M2 |
| `show <feature> [<artifact>] [--rev N] [--task <key>]` | `--iter`, `--file`, `--run`, `--json`: M2 |
| — | `stop`, `inspect`, `logs`, `restore`, `delete`: M2 |

Every command and flag marked M2 above now exists: Milestone 2 implemented them ([After Milestone 1](#after-milestone-1)).

### Mechanisms deferred

| Mechanism | Milestone 1 behaviour | Full design | Arrives | Migration? |
|---|---|---|---|---|
| Pause on Transient Failure | A `transient` failure blocks the Step Run at once without spending an Attempt: Execution `failed` with `failure_class = 'transient'`, Step Run `blocked` with `blocked_reason = 'not_retryable'`, exit `4`. The message names the reset time when the stream gives one. | [I. Recovery](I-recovery.md#transient-failures-network-and-rate-limits) | M3 | no |
| `[git] merge_into_base = "ff-only"` | `feature.toml` has no `merge` step. Only `"never"` is accepted; `"ff-only"` refuses with exit `3`. Frozen definitions keep old runs unaffected when the step is added. | [A. Git Integration](A-git-integration.md), [ADR 0006](../adr/0006-merge-into-base-is-opt-in-fast-forward.md) | M4 | no |
| `[cleanup] on_success = "keep"` | Only `"clean"` parses. A `succeeded` run always cleans. | [I. Cleanup](I-cleanup.md#what-each-ending-removes) | M4 | no |
| Agent overrides in `.surt/agents/` | Only the built-in `Catalog`. Driving commands refuse with exit `3` when `.surt/agents/` exists, rather than ignoring it. `init` still lists `agents/` as versioned. | [F](F-agent-contract.md#built-in-defaults-and-local-overrides) | M4 | no |
| Global `~/.surt/config.toml` | Not read. `stop_grace_period` is fixed at 10 s. | [J](J-cli.md#global-surtconfigtoml-optional) | M3 | no |
| Agent slots across processes | `max_concurrent_agents` is a per-Driver semaphore. Two Features driven at once may use 3 + 3. `status` omits `agent slots`. `execution.slot` is still written. | [A. Process Model](A-process-model.md#agent-concurrency-limit) | M3 (implemented) | no |
| TTY footer, 5-minute line without a TTY, ASCII symbols | Event lines only. | [J](J-cli.md#foreground-progress) | M2 (implemented) | no |

**Kept in Milestone 1** although they could have been cut: the whole Recovery of [I](I-recovery.md#what-a-driving-command-does-when-it-takes-the-lock), Salvage Refs (worktree preparation is one code path for the normal flow and recovery, and salvage protects human work for a handful of Git commands), Checks, cleanup with its pending retry, and `cleaned_at`.

### Build order

Each slice is one PR into `main` with CI green on the three OSes and a demonstrable "done".

```
S1 Skeleton ──► S2 Workflow ──► S3 Engine + Fake ──► S4 Git ──► S5 Recovery and ending
                                        │
                                        └──► S6 Claude Code (in parallel once S3 fixes the trait)
```

| Slice | Contents | Done when |
|---|---|---|
| **S1 Skeleton** | Two-crate workspace replacing the `ferris-says` crate, `rust-toolchain.toml`, `.github/workflows/ci.yml` with the matrix (fmt, clippy `-D warnings`, test), `platform` (locks, process identity), `store` (WAL + `synchronous=FULL`, `0001_initial.sql`, `BEGIN IMMEDIATE` with retry), `Project::open`, `surt init` | `surt init` works and is idempotent on the three OSes |
| **S2 Workflow** | Parsing the five `kind`s with `deny_unknown_fields`, static validation, freezing and SHA-256, the `Catalog` with `feature.toml`, `task-cycle.toml` and the five Agents (TOML, instructions, schemas), `create_run`, `YYYYMMDD-<slug>` naming | `surt feature "add login"` creates the Subject and a `pending` run, and stops without driving |
| **S3 Engine + Fake** | `decide()` with table tests; `drive` (tick, lock, Iterations, Per-Item, limits, Ctrl-C); `artifacts` (Revisions, Working Copy, edits, pinning); `context`; the `AgentRuntime` trait, `FakeRuntime` and the `test-support` feature; `approve`/`reject` | spec → gate → reject → spec rev 2 → approve → `plan` → gate, with the Fake, through the binary |
| **S4 Git** | `git` (argv, minimum version, `git.lock`), five Operations (not `git.merge_into_base`), worktree preparation and salvage, `start` | the brief's E2E is green (without `kill`), plus the three Git scenarios |
| **S5 Recovery and ending** | `reconcile` (the seven steps, `ref_move`, pending Working Copy values), fault points at the eight checkpoints, the `kill` smoke, `cleanup` with retry, `cancel`, detailed `status`, `show` | the whole suite of the test strategy passes |
| **S6 Claude Code** | `ClaudeCodeRuntime`: launcher argv, environment allowlist, stream-json parser, failure classification, process trees; the fixtures copied from branch `research/claude-code-cli-headless` into `tests/fixtures/` | the parser passes against the fixtures and the manual macOS smoke passes; Milestone 1 is done |

Alternative rejected: thin vertical slices ("one Task without review, end to end" first). Recovery and worktree preparation cut across every step, so each slice would rewrite them.

### Files

The module split of [B](B-workspace.md#modules-of-surt-engine) is fixed; file names inside a module may change.

```
Cargo.toml  rust-toolchain.toml  .github/workflows/ci.yml
crates/surt-engine/
├── migrations/0001_initial.sql          # docs/architecture/migrations/0001_initial.sql, unchanged
├── schemas/{base,check-report,conflict,integration}.schema.json
├── src/
│   ├── lib.rs  project.rs  artifacts.rs  context.rs  operations.rs  git.rs  cleanup.rs  reconcile.rs
│   ├── workflow/{mod,parse,validate,freeze}.rs
│   ├── drive/{mod,decide}.rs            # decide() is pure, apart from the loop
│   ├── runtime/{mod,claude_code,fake}.rs
│   ├── store/{mod,checkpoints,views}.rs # one method per checkpoint
│   └── platform/{mod,unix,windows}.rs   # every #[cfg] lives here
└── tests/{decide,store,git_scenarios,recovery,claude_stream}.rs  tests/fixtures/claude-code/…
crates/surt/
├── assets/workflows/{feature,task-cycle}.toml
├── assets/agents/{spec,tasks,builder,reviewer,shipper}/{agent.toml,instructions.md,schemas/…}
├── src/{main,builtin,render,exit}.rs    # render: ✔ ✖ ⚠ text; exit: DriveOutcome/Refusal → code
├── src/commands/{init,feature,plan,start,approve,reject,cancel,status,show}.rs
└── tests/e2e.rs  tests/scripts/*.toml   # Fake scripts for the binary
docs/smoke-claude-code.md                # manual smoke checklist
```

`docs/architecture/schemas/*.schema.json` move to the crate that owns them: Operation outputs to `surt-engine/schemas/`, agent outputs (`manifest`, `commit`, `feedback`, `delivery`) to their Agent in `surt/assets/agents/`.

## After Milestone 1

| Milestone | Goal | Contents | Status |
|---|---|---|---|
| **M2 Operate and debug** | understand and fix a run without opening SQLite | `inspect`, `logs [-f] [--raw]`, full `show`, `--json` on reads, `--run`, `stop`, `restore`, `delete`, `--new-run`, the TTY footer, the 5-minute line, ASCII symbols | implemented; shipped once CI passes on macOS, Linux and Windows and the manual macOS smoke passes |
| **M3 Run unattended** | leave a long `surt start` running without coming back to restart it | the Transient Failure pause (reset time and backoff), agent slots across processes, `~/.surt/config.toml` | next |
| **M4 Fit the project** | adapt agents and delivery to the repository | `.surt/agents/` overrides, `merge_into_base = "ff-only"` with `--merge`/`--no-merge`, `on_success = "keep"` | planned |

**Milestone 2 is implemented** with the spec [Milestone 2](../../.scratch/milestone-2/spec.md): a run is operated and debugged through `inspect`, `logs`, `show`, `--json`, `stop`, `restore`, `delete` and `--new-run`, and every M2 change to sections B, D, H, I and J is written there. As with Milestone 1, it ships once `cargo test --features test-support` passes in CI on macOS, Linux and Windows and the manual smoke with the real Claude Code ([`docs/smoke-claude-code.md`](../smoke-claude-code.md)) passes on macOS. M3 is next.

**v1 is the end of M4**: everything sections A–J describe. M2 comes before M3 because a blocked run in Milestone 1 already says what to do, while debugging a review rejected three times with only `tail` hurts every time; a rate limit blocks with a time to come back, which is rarer. What the map ruled out of scope stays out.
