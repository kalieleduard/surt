# 13: Checks and Builder commit evidence

Spec: [Milestone 1](../spec.md) (stories 46–49) · Slice: S4 Git · Plan: [A. Git Integration](../../../docs/architecture/A-git-integration.md), [F](../../../docs/architecture/F-agent-contract.md)

**What to build:** a Builder's delivery counts only if Git proves the work, and the project's frozen `[[checks]]` run on that commit before any Reviewer time is spent. A failed Check starts a new Iteration whose Builder reads the `check-report`.

**Blocked by:** 12 (Git adapter, Integration Branch and Task worktrees)

**Status:** resolved

- [x] A Builder delivery is accepted only if its commit exists, is the worktree's `HEAD` and the Task Branch tip, descends from the Task's start, is new, and leaves a clean worktree; a SHA-shaped string alone is refused as an invalid delivery
- [x] An empty commit explaining why nothing changed is a valid delivery
- [x] `checks.run` runs each frozen Check as argv, without a shell, in the Task worktree on the Builder's commit, runs all of them even after one fails, and honours the optional timeout
- [x] A failed Check skips review and starts a new Iteration whose Builder receives the `check-report`
- [x] `Project` tests against a real repository: valid commit, each evidence refusal, the failing-Checks scenario

## Comments

- 2026-10-18 (orchestrator): Resolved. Merged into `integration/milestone-1` at 483dffb; code-review fixes at 931fc99.
