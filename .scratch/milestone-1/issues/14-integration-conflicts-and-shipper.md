# 14: Integration, conflicts and the Shipper

Spec: [Milestone 1](../spec.md) (stories 50–57) · Slice: S4 Git · Plan: [A. Git Integration](../../../docs/architecture/A-git-integration.md), [ADR 0007](../../../docs/adr/0007-integrate-reviewed-sha-as-merge-commit.md)

**What to build:** an approved Task enters the Integration Branch as a merge commit of exactly the reviewed SHA, computed without a worktree, and every tip of the Integration Branch passes the Checks. A conflict or failed re-check sends the Task back to its Builder; once every Task merged, the Shipper writes the delivery text. This ticket proves the three Git scenarios of the test strategy.

**Blocked by:** 13 (Checks and Builder commit evidence)

**Status:** ready-for-agent

- [x] The Reviewer's verdict is bound to the reviewed SHA (`reviewed_commit` = `commit.sha`); `approved` with a blocking finding is an invalid delivery; `changes_requested` starts a new Iteration whose Builder reads the `feedback`, and the next Reviewer verifies earlier blocking findings
- [x] `git.integrate`: `merge-tree --write-tree` → `commit-tree -p T -p R` dated by the Step Run's start → rerun the Checks when the tip moved since the Task started → `ref_move` Intention → `update-ref` with the expected old value → confirmation
- [x] The Integration Branch only moves from the tip Surt recorded
- [x] A conflict or failed re-check sends the Task back with a `conflict` artifact; the Builder's next commit must also descend from the conflict's integration tip
- [x] `git.snapshot_run_branch` records the integration snapshot; the Shipper runs only after every Task merged, reads the manifest and the snapshot, delivers `title` and `summary` and touches no Git
- [x] `Project` tests against a real repository: clean merge, conflict back to the Builder, failing Checks; every Task's reviewed SHA is a parent of a merge commit on the Integration Branch
