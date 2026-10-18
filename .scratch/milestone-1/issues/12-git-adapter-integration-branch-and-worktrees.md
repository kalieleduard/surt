# 12: Git adapter, Integration Branch and Task worktrees

Spec: [Milestone 1](../spec.md) (stories 42–43, 75–76) · Slice: S4 Git · Plan: [A. Git Integration](../../../docs/architecture/A-git-integration.md), [I. Preparing a worktree](../../../docs/architecture/I-recovery.md#preparing-a-worktree-for-every-attempt), [ADR 0010](../../../docs/adr/0010-every-attempt-starts-from-a-clean-worktree.md)

**What to build:** Surt owns an Integration Branch `surt/<feature>` created at the Base, and every Task runs in its own worktree on its own Task Branch `surt/task/<feature>/<key>` starting from the Integration Branch tip. Every Attempt in a worktree starts clean at the last accepted tip, with any leftovers saved to a Salvage Ref so nothing is lost; the same code serves the normal flow and recovery.

**Blocked by:** 11 (Per-Item, limits, `blocked` and Ctrl-C in `drive`)

**Status:** resolved

- [x] The `git` CLI runs via argv with `LC_ALL=C`, enforces the minimum version, and backs off on third-party `git.lock`s
- [x] Operations are a closed enum; `git.create_run_branch` and `git.create_child_worktree` follow "read state → compare with SQLite → act", so a repeated Attempt is its own reconciliation
- [x] `git.create_child_worktree` (and later `git.integrate`) is serialized per root run
- [x] Worktrees live under `~/.surt/worktrees/<project-id>/<feature>/<key>`, created with `worktree add --lock`; on Windows, Builders get `core.longpaths=true` via `GIT_CONFIG_*`
- [x] Preparation: locate → remove Surt's own locks → save leftovers to `refs/surt/salvage/<execution-id>` with a temporary index → clear in-progress operations → `checkout --force -B` + `clean -fd` → verify, with a warning naming the Salvage Ref
- [x] A missing, half-created or orphaned worktree is repaired; a path Surt didn't create blocks instead of being deleted
- [x] `Project` tests against a real temporary repository: fresh worktree, leftovers salvaged, half-created worktree repaired, foreign path blocks

## Comments

- 2026-10-18 (orchestrator): Resolved. Merged into `integration/milestone-1` at daf89d4; code-review fixes at 931fc99.
