# Roadmap + Milestone 1 proposal

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Blocked by: 13, 15, 16

## Question

What is the smallest Milestone 1 that already delivers a Feature end to end, and what comes after?

Decide:
- **M1:** goal, Rust modules, files, tests and definition of done. Natural criterion: the brief's E2E passes with the Fake Runtime and a small Feature passes with real Claude Code.
- **Command cut:** of the 14 in [J](../../../docs/architecture/J-cli.md), which go into M1. Candidates to leave out: `logs`, `inspect <execution-id>`, `--json`, live footer, `--new-run`, `stop` — without breaking exit codes and messages.
- **Cut of already decided mechanisms:** what was designed in the resolved tickets but can wait without changing the schema (e.g. pause on Transient Failure, `merge_into_base`, Salvage Refs, `restore`). For each one, confirm that deferring it does not require a destructive migration.
- **Following milestones:** order and goal in one line each.

Cut candidates coming from [I. Cleanup](../../../docs/architecture/I-cleanup.md): `on_success = "keep"`, salvage before removing a worktree on `cancel`, `surt delete` (and the concurrent `--new-run` check). `cleaned_at` and `.surt/logs/` are already in the schema/layout; deferring requires no migration.

From [B. Rust Workspace](../../../docs/architecture/B-workspace.md): M1's modules come from B's module table (each may land partially); the natural build order is `platform`/`store`/`workflow` → `decide` + `drive` with `FakeRuntime` → `operations`/`git` → `reconcile`/`cleanup` → `ClaudeCodeRuntime`. B's dependency list is M1's ceiling, not its floor.

Output: sections K and L (pending ADRs consolidated). Closes the map.

## Answer

Resolved in a grilling session (two rounds, all recommendations accepted). Output: [K. Roadmap and Milestone 1](../../../docs/architecture/K-roadmap.md) and [L. Architecture Decision Records](../../../docs/architecture/L-decisions.md). "Milestone 1" in A, G and J, where it meant the whole v1, became "v1"; J points to K for the M1 subset.

Decisions:

- **Goal and done (Q1):** "a small Feature goes from `surt feature` to a delivered Integration Branch, with real Claude Code, and survives `kill -9`". Done = the [Test strategy](16-test-strategy.md) suite green in CI on the three OSes + a manual smoke test with real Claude Code **on macOS only** (a 2-Task Feature up to `succeeded`, checklist in `docs/smoke-claude-code.md`).
- **Commands (Q2):** 10 commands — `init`, `feature` (create with `--name`/`--base`/`--new`, resume), `plan`, `start`, `approve`, `reject`, `cancel`, `status` (no `--json`), a lean `show` (`--rev`, `--task`). Out: `stop` (Ctrl-C), `inspect`/`logs` (`status` shows the log path of the blocked step), `restore` (deleting the file recreates it; the refusal says `fix: rm <file>`), `delete`, `--new-run`, `--json`, `--merge`/`--no-merge`. Exit codes and message format intact.
- **Visible mechanisms deferred (Q3):** a Transient Failure blocks immediately without spending an Attempt (`blocked_reason = 'not_retryable'`, exit 4, reset time in the message); `merge_into_base` only `"never"` (no `merge` step in `feature.toml`); `on_success` only `"clean"`. Kept: full recovery, Salvage Refs, Checks, cleanup with retry.
- **Internal mechanisms deferred (Q4):** overrides in `.surt/agents/` (driving commands refuse if it exists), global config (`stop_grace_period` fixed at 10 s), cross-process slots (one semaphore per Driver), footer/5-min line/ASCII.
- **Shape of K and L (Q5):** K = roadmap + M1; L = index of ADRs 0001–0012, no new ADR — M1's cuts are reversible subsets with no migration.
- **Slices (Q6):** S1 Skeleton → S2 Workflow → S3 Engine + Fake → S4 Git (E2E green) → S5 Recovery and ending; S6 Claude Code in parallel starting from S3. One PR per slice with green CI.
- **Files (Q7):** K's tree; [B](../../../docs/architecture/B-workspace.md)'s module split locked, file names free.
- **After M1 (Q8):** M2 Operate and debug → M3 Run unattended → M4 Fit to the project; **v1 = end of M4**.

Every cut checked against `0001_initial.sql`: none requires a migration.
