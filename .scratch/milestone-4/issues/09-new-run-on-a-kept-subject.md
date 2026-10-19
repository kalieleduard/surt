# 09: `--new-run` on a kept Subject

Spec: [Milestone 4](../spec.md) (stories 69, 70) · Plan: [I. Cleanup](../../../docs/architecture/I-cleanup.md), [J. `surt feature`](../../../docs/architecture/J-cli.md)

**What to build:** a developer who kept a successful run starts run 2 of the same Subject to compare the two. The kept run's worktrees and branches stay where they are, under their own names, while the Working Copy moves on to the new run so that two runs never share it.

**Blocked by:** 08 (`on_success = "keep"` keeps a successful run)

**Status:** resolved

- [x] Before creating run `N+1`, `--new-run` clears the previous kept run's Working Copy files that equal their baseline (the rule of cleanup step 1) and removes the directories left empty
- [x] A Working Copy file with an unrecorded edit refuses with exit `3`, as for any previous run
- [x] The kept run's worktrees, Task Branches, Salvage Refs and Execution directories are untouched, and reconciling run `N+1` never touches them
- [x] `Project` tests with real Git: `--new-run` on a kept Subject clears the baseline files, leaves the kept run's worktrees and refs, and run `N+1` drives with its own `.N` names; an unrecorded edit refuses
- [x] I records the `--new-run` rule for a kept Subject

## Comments

- 2026-10-18 (orchestrator): Implemented and green on Windows. Merged into `integration/milestone-4` at 8e41227; code-review fixes at 863ebc6. Resolved.
