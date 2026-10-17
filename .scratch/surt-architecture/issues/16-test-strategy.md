# Test strategy

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Blocked by: 07, 10, 12
Assignee: Kaliel

## Question

How do we test Surt without spending tokens, with enough confidence to ship M1?

Decide, for v1 only:
- **Fake Agent Runtime:** how each agent's script is written (valid delivery, invalid delivery, rejected review, failure) and how a test picks the script.
- **Crash between intention and effect:** a single mechanism — fault points in the code or actually killing the process — and which checkpoints of [I](../../../docs/architecture/I-recovery.md) are covered in M1.
- **Git:** a real temporary repository per test; minimum scenarios: clean merge, conflict, failing Checks.
- **The brief's E2E:** feature → spec → approval → 2+ tasks → builders → rejected review → new builder → approval → shipper → cleanup → verify the final state. Where it runs and how long it may take.
- **CI:** macOS/Linux/Windows matrix — what runs on each.

Out of v1 (do not decide here): testing every row of the recovery tables, double Ctrl-C, concurrent migrations, a Git version matrix, ASCII symbols without UTF-8, snapshots of every message, the date rollover in the slug.

## Context

- **Builder ↔ Reviewer loop:** the main behavior to prove with the Fake — 3 invalid deliveries → `blocked`, running again renews the budget; a rejected review opens a new Iteration with `feedback`; a blocked Task holds back only its dependents ([G](../../../docs/architecture/G-state-machines.md)).
- **Concurrency:** a second Driver on the same run refuses; `kill -9` releases locks ([A-process-model](../../../docs/architecture/A-process-model.md)).
- **SQLite:** tests against a real file (not in-memory); the CHECKs/unique indexes that uphold invariants (≤1 active root per Subject, ≤1 live Execution per Step Run) ([D](../../../docs/architecture/D-database.md)).
- **CLI:** each exit code (0, 2, 3, 4, 5, 130) and that reading changes nothing ([J](../../../docs/architecture/J-cli.md)).
- **Cleanup:** the E2E ends by checking the final state of [I. Cleanup](../../../docs/architecture/I-cleanup.md#what-each-ending-removes) — worktrees and Task Branches gone, Integration Branch and `.surt/logs/<subject>/` present, `cleaned_at` written; a removal failure leaves `cleanup pending` and the next command finishes it; an interrupted `surt delete` finishes when run again.
- **Real fixture:** when [19](19-real-claude-code-stream-fixture.md) lands, the captured stream becomes a test of the Claude Code Runtime's parser; it does not block the Fake's design.
- **Test seams (from [B](../../../docs/architecture/B-workspace.md)):** only `AgentRuntime` has a trait; `FakeRuntime` is public in `surt-engine::runtime`, and how the `surt` binary selects it in an E2E is still to be decided. Store and Git are real in a tempdir; timers with `tokio::time::pause()`. The rules of G are tested on the pure function `decide(&RunGraph, &Ctx)`. Engine tests use synthetic workflows (`feature.toml` does not exist in the engine crate).

## Answer

Resolved in a grilling session (three rounds, all accepted). No new artifact in `docs/architecture/`: the decision is about the testing process, not a contract, and `B-workspace.md` already points to this ticket.

Decisions:

- **Fake script (Q1):** engine tests use a typed script in Rust, indexed by the step's logical key + Iteration + Attempt (`Script::new().on("build", 1, 1, Behavior::Commit)`). A TOML file per scenario only when the `surt` binary needs to receive the script from outside. Behaviors: those of [F. Agent Contract](../../../docs/architecture/F-agent-contract.md#agent-runtime-and-fake-runtime).
- **Crash between intention and effect (Q2):** fault points in the store, covering the eight checkpoints of [I. Recovery](../../../docs/architecture/I-recovery.md#transactional-checkpoints). A real `kill` smoke test complements them (Q8). No other mechanism in M1.
- **Git (Q3):** a real temporary repository per test; only the ticket's three scenarios (clean merge, conflict, failing Checks). Partial Git states are covered by injecting failures at the checkpoints, with no new repositories.
- **CI (Q4):** the whole suite runs on the three OSes with the Fake Runtime and real Git (≥ 2.38, pinned per runner). Real Claude Code stays out of CI and becomes a manual smoke test with the fixture from [19](19-real-claude-code-stream-fixture.md). There is no `.github/workflows` today; CI is an M1 deliverable. Before the tests, each OS runs `cargo fmt --all -- --check` and `cargo clippy --all-targets --features test-support -- -D warnings` (added in the review against `rules/rust.md`, 2026-10-17).
- **E2E (Q5):** runs through the `surt` binary, covering exit codes 0/2/3/4/5/130 along with the brief's flow.
- **Fake selection (Q6):** a cargo feature `test-support`, off by default, gates selecting the Fake through `SURT_RUNTIME` (with the script's path) and the fault points. Release never enables it; CI and `cargo test` turn it on.
- **Budget (Q7):** E2E capped at 60 s per OS, with no real `sleep`. Going over is a signal to cut a scenario.
- **`kill` smoke test (Q8):** `kill` during a Fake Execution that is waiting (state `running`, deterministic). Resuming must finish the run, with the dead Execution marked `interrupted` and a new Attempt, as in [G](../../../docs/architecture/G-state-machines.md).

Kept out of v1 (unchanged): testing every row of the recovery tables, double Ctrl-C, concurrent migrations, a Git version matrix, ASCII symbols without UTF-8, snapshots of every message, the date rollover in the slug.
