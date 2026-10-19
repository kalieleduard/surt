# A. Git Integration

How code flows from a Builder's commit to the branch the Shipper delivers, and how Surt proves that nothing unreviewed gets in. Vocabulary (**Base**, **Integration Branch**, **Task Branch**, **Check**, **Operation**) is defined in [`CONTEXT.md`](../../CONTEXT.md). Decision ticket: [Git integration model](../../.scratch/surt-architecture/issues/09-git-integration-model.md). Facts about Git behaviour come from [Git worktree/merge mechanics and partial states](../../.scratch/surt-architecture/issues/04-git-worktree-merge-partial-states.md), whose full report is on branch `research/git-worktree-merge`.

## Shape of the flow

```
main @ a11 ─────────────────────────────────────────────── (ff-only, opt-in)
  │ Base                                                       ▲
  ▼                                                            │
surt/20261016-add-login   a11 ── M_A ── M_C ── M_B ── snapshot ┘
                           ├─ A ──┘      │      │
                           ├─ C ─────────┘      │
                           └──── (from M_A) B ──┘
surt/task/20261016-add-login/{A,B,C}
```

1. `surt feature` fixes the **Base**: the commit at `HEAD`, or at `--base <ref>`.
2. `prepare` creates the **Integration Branch** at the Base.
3. Each Task starts its **Task Branch** from the Integration Branch's tip when its `setup` runs. A Task whose `needs` are merged therefore sees their code.
4. The Builder commits. Surt runs the **Checks** on that exact commit, and the Reviewer reviews that exact commit.
5. `integrate` merges the **reviewed SHA** into the Integration Branch as a merge commit. Before moving the ref, Surt runs the Checks again on the merge result if the tip moved since the Task started.
6. `snapshot` fixes the final tip. The Shipper writes the delivery text, and `merge` fast-forwards the Base branch only if the project opted in.

Surt never pushes, never rewrites history, never commits artifacts such as the spec, and never merges into the main branch unless `[git] merge_into_base = "ff-only"` is set ([ADR 0006](../adr/0006-merge-into-base-is-opt-in-fast-forward.md)).

## Prerequisites and preflight

Checked by `surt feature` and before every driving command:

- `git` on `PATH` (`git.exe` on Windows), version **≥ 2.38**, required by `merge-tree --write-tree`. Ubuntu 22.04 ships 2.34 and needs an upgrade; Debian 12, Ubuntu 24.04 and current Xcode Git qualify.
- The project root, the directory holding `.surt/`, is the top level of a non-bare worktree.
- A commit identity resolves (`git var GIT_COMMITTER_IDENT`). Surt's merge commits use it.

```
✖ git 2.34.1 is too old; surt needs git 2.38 or later
✖ git has no user identity; run: git config user.email "you@example.com"
```

Surt uses only the `git` CLI, with argument vectors and no shell. libgit2 and gix are not used: neither offers `worktree repair`, libgit2's merge is not `ort`, and gix is still early.

## Names and paths

### Branches

| Branch | Name | Created by |
|---|---|---|
| Integration Branch | `surt/<subject>` | `git.create_run_branch` |
| Task Branch | `surt/task/<subject>/<key>` | `git.create_child_worktree` |

- Git cannot store `refs/heads/surt/X` and `refs/heads/surt/X/A` at once. Task Branches therefore live under `surt/task/`, which never collides with a Subject because every Subject starts with digits.
- From the second root run of the same Subject, `<subject>` becomes `<subject>.N` in branch names and worktree paths, where `N` is the run's ordinal within its Subject: `surt/20261016-add-login.2`, `surt/task/20261016-add-login.2/A`. No run renames or deletes another run's branches. A `.` never occurs in a Subject slug, so `.2` cannot be mistaken for the Subject `20261016-add-login-2`.
- Every generated name is also checked with `git check-ref-format --branch` as a defence.

### Project id and worktrees

```
~/.surt/worktrees/<project-id>/<subject>[.N]/<key>/
~/.surt/worktrees/surt-3fa91c/20261016-add-login/A/
```

- `~/.surt` is the global Surt home (`%USERPROFILE%\.surt` on Windows).
- `<project-id>` is generated once per checkout, when `.surt/surt.db` is created, and stored there: the project directory name reduced by the Subject slug rules to at most 24 characters (`project` if empty), a hyphen, and 6 random lowercase hex digits. A hash of the path would change when the repository moves, and an id in the versioned `config.toml` would be shared by two clones.
- The absolute path of each worktree is recorded in SQLite when it is created. Surt never recomputes where a worktree "should" be.
- Inputs never enter a worktree. A Builder reads them from `.surt/runtime/executions/<id>/inputs/` ([H. Artifact Storage](H-artifact-storage.md)).

### Item keys and item file paths

Item keys become path components of branch names and directories, and they are used literally, without encoding. The engine enforces these rules for **every** Item List when its producer delivers it and again on expansion. A violation is an invalid delivery and leads to a new Attempt:

- A key matches `^[A-Za-z0-9][A-Za-z0-9_-]{0,31}$`.
- Keys are unique **case-insensitively** (`a` and `A` would share a directory on APFS and NTFS).
- A key is not a Windows reserved device name in any case: `CON`, `PRN`, `AUX`, `NUL`, `COM0`–`COM9`, `LPT0`–`LPT9`.

```
✖ plan-tasks attempt 1: invalid item list — key "con" is a reserved name on Windows
↻ plan-tasks attempt 2
```

Item file paths (`files.task = "tasks/A.md"`) are portable relative paths ([F. Agent Contract](F-agent-contract.md#one-markdown-document-per-task)). Each path component also matches `[A-Za-z0-9._-]+`, is not `.` or `..`, has no trailing dot, and does not have a reserved device name as its stem (`con.md` is refused). Full paths are unique case-insensitively within one Revision. The Working Copy (`.surt/features/<subject>/tasks/A.md`) and `inputs/` mirror these paths verbatim.

## The Base

`surt feature` resolves the Base before the run is created, and stores it as a human-origin Revision of the initial artifact `base`, next to `request` ([schema](schemas/base.schema.json)):

```json
{ "ref": "main", "sha": "a11c9d0e5f…" }
```

- `ref` is the short branch name checked out at `HEAD`, or the branch given to `--base`. It is `null` when the Base is a detached commit or a tag; then `merge_into_base` always skips.
- Only commits count. Uncommitted changes are not part of the Base, and Surt says so:
  ```
  $ surt feature "add login"
  ⚠ uncommitted changes in your worktree are not part of base a11c9d0
  ✔ created 20261016-add-login   base: main @ a11c9d0
  ```
- The Base is shown by `status`, and a later run of the same Subject resolves its own Base when it is created.

The engine does not know what a Base is. It accepts **initial artifacts** supplied by the command that creates a root run, as human-origin Revisions; `request` and `base` are the two the feature workflow uses ([E. Workflow Definition Schema](E-workflow-schema.md#artifacts-and-inputs)).

## Operation registry

Operations are referenced from TOML only by id ([E](E-workflow-schema.md#step-kinds)). Ids use engine names: a **Run Branch** belongs to any Workflow Run; the feature workflow calls its root's Run Branch the Integration Branch and a Task Cycle's the Task Branch. They take no arguments from the TOML. Each one reads its Workflow Run's context: Subject, run ordinal, current `item`, its fixed Input Revisions, and the run's frozen project configuration. Each Operation launch is an Execution; a technical failure leads to a new Attempt, which first reads Git state and compares it with SQLite (partial states: [I. Recovery and Reconciliation](I-recovery.md#operations)).

| Id | Used by | Inputs | Does | Outcomes | Produces |
|---|---|---|---|---|---|
| `git.create_run_branch` | feature / `prepare` | `base` | Creates `surt/<subject>[.N]` at `base.sha`. | `done` | — |
| `git.create_child_worktree` | task-cycle / `setup` | `item` | Reads the Integration Branch tip and records it as the Task's **start** (the `run_branch` row, written before the branch as its Intention), then creates the Task Branch there and adds its worktree with `--lock`. | `done` | — |
| `checks.run` | task-cycle / `check` | `commit` | Runs the frozen Checks in the Task worktree on `commit.sha`. | `passed` · `failed` | `check-report` |
| `git.integrate` | task-cycle / `integrate` | `commit`, `feedback` | Merges the reviewed SHA into the Integration Branch (below). | `merged` · `conflict` | `conflict` (on `conflict`) |
| `git.snapshot_run_branch` | feature / `snapshot` | `base` | Verifies the Integration Branch and fixes its final tip. | `done` | `integration` |
| `git.merge_into_base` | feature / `merge` | `base`, `integration` | Fast-forwards the Base branch to the delivered tip, if enabled. | `merged` · `skipped` | — |

Locks ([A. Process Model](A-process-model.md#native-git-operations-across-processes)): worktree add/remove, branch creation, `merge-tree`, `commit-tree` and `update-ref` run under `.surt/runtime/locks/git.lock`. Inside one Driver, the start-tip read of `git.create_child_worktree` and all of `git.integrate` are additionally **serialized per root run**: one at a time per Feature. Without that, two Tasks re-checking in parallel would invalidate each other's compare-and-swap. A Driver is unique per root run, so an in-process mutex suffices.

## Checks

```toml
# .surt/config.toml
[[checks]]
name = "test"
run = ["cargo", "test", "--workspace"]
timeout = "10m"            # optional

[[checks]]
name = "lint"
run = ["cargo", "clippy", "--workspace", "--", "-D", "warnings"]
```

- `run` is an argument vector, executed without a shell on every platform. Pipelines and `&&` belong in a script that `run` points to. `name` follows the identifier rules of agent ids.
- Checks are part of the project configuration **frozen at root run creation**, like agents ([F](F-agent-contract.md)). They are never read from the `.surt/config.toml` inside a worktree, so a Builder cannot disable the Check that fails.
- `checks.run` runs every Check in declaration order with the Task worktree as working directory, even after one fails, so the Builder sees all failures at once. It is an Operation and takes no agent slot. A Check that exceeds its `timeout` is stopped like any child process tree ([A](A-process-model.md)) and counts as failed. With no `[[checks]]`, the Outcome is `passed` with an empty report.
- `check-report` ([schema](schemas/check-report.schema.json)) records the tested SHA and, per Check, its status, exit code, duration and the last 200 lines of combined output. The full output is the Execution's `output.log` ([I. Cleanup](I-cleanup.md#logs)).
- `failed` skips `review` and `integrate` (`when = { step = "check", outcome = "passed" }`). A new Iteration starts, and the Builder reads that `check-report`. No Reviewer time is spent on a commit that does not pass.

```
  A  iter 2  build ✔  check ✖ (test: 3 failed)  review –  integrate –   → iter 3
```

## Evidence for a Builder's commit

The Builder delivers `commit` = `{ "sha": "<full sha>" }` ([schema](schemas/commit.schema.json)). Matching a SHA regex proves nothing, so the built-in workflow registers a consistency check on that output through generic delivery validation. It requires, in the Task worktree:

1. `git rev-parse --verify <sha>^{commit}` succeeds.
2. `<sha>` is the worktree's `HEAD` **and** the tip of its Task Branch: the Builder did not switch branches or leave a detached `HEAD`.
3. `<sha>` descends from the Task's start, and also from `conflict.integration_tip` when this Iteration follows a conflict (`git merge-base --is-ancestor`). It differs from the commit the Iteration started at: there is at least one new commit.
4. The worktree is clean (`git --no-optional-locks status --porcelain=v2 --untracked-files=all` is empty; ignored files may exist), and no merge, rebase, cherry-pick or revert is in progress (markers found via `rev-parse --git-path`: `MERGE_HEAD`, `rebase-merge/`, `rebase-apply/`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`, `sequencer/`).

A failed check is an invalid delivery, so a new Attempt follows ([F](F-agent-contract.md#delivery-files-and-a-structured-result)). There is no "no changes" Outcome: a Builder that finds nothing to change makes an empty commit explaining why, which the Reviewer judges ([G. State Machines](G-state-machines.md#a-builder-with-nothing-to-change)).

A Builder may make several commits; the review covers the whole range from the Task's start to `<sha>`.

## Binding a review to a SHA

- `checks.run` writes the SHA it tested into `check-report.commit`.
- The Reviewer's `feedback.reviewed_commit` must equal its Input `commit.sha`, and the `check-report` Input must be for that same SHA. Otherwise the delivery is invalid.
- `git.integrate` merges **the approved SHA** (`feedback.reviewed_commit`), never the Task Branch by name. Commits added to a Task Branch after its review, by a human or an orphaned agent, are ignored.
- **What invalidates an approval:** only a new SHA. Every new commit needs new Checks and a new review. The Integration Branch moving does not invalidate a review, because integration re-checks the merge result (below).

The SHA covers content and ancestry. What gets committed on Windows depends on the user's `core.autocrlf`; Surt binds to the committed bytes and does not impose a line-ending policy.

## Integration

`git.integrate` for a Task whose review is `approved`, with `R` = the reviewed SHA, `S` = the Task's start and `T` = the Integration Branch tip recorded in SQLite:

1. **Ownership.** The Integration Branch must not be checked out in any worktree (`git worktree list --porcelain -z`), and the ref must equal `T`. Otherwise Surt refuses (below).
2. **Merge without a worktree.** `git merge-tree --write-tree -z T R`. Exit status `1` means a conflict: Surt records `conflict` with `kind = "merge"` and the conflicting paths, and the Outcome is `conflict`. It never commits the returned tree, which contains conflict markers.
3. **Commit.** `git commit-tree <tree> -p T -p R` with a fixed message and with author and committer dates set to the recorded start of the `integrate` Step Run (`step_run.started_at`, the first time it became running, which never changes). The resulting SHA `M` is therefore deterministic: every Attempt of that Step Run, each a new Execution with its own start time, recomputes the same `M`. It is always a merge commit, even when a fast-forward would be possible, so `R` stays in history and `git merge-base --is-ancestor R surt/<subject>` proves it was integrated ([ADR 0007](../adr/0007-integrate-reviewed-sha-as-merge-commit.md)).
4. **Re-check if the tip moved.** If `T = S`, the tree of `M` equals the tree that already passed and Surt skips this step. Otherwise it runs `git checkout --detach M` in the Task worktree (the Builder is done and its build cache is warm), runs the frozen Checks, and checks the Task Branch out again. A failure records `conflict` with `kind = "checks"` and the failing report, and the Outcome is `conflict`. `M` stays unreferenced.
5. **Move the ref.** Commit the Intention (`ref_move` T → M), then `git update-ref --stdin` with `start` / `update refs/heads/surt/<subject> M T` / `commit`, under `git.lock`, then record `M` as the new tip and delete the Intention. On recovery: ref = `M` means done, ref = `T` means redo, anything else means stop ([I](I-recovery.md#ref-moves)).

Invariant: **every tip of an Integration Branch passed the Checks, and every Task merged into it has its reviewed SHA as a parent of a merge commit.**

Merge commit format:

```
Merge task C into surt/20261016-add-login

Surt-Run: 20261016-add-login
Surt-Task: C
Surt-Reviewed-Commit: c9d4e1f…
```

`commit-tree` runs no hooks. A Builder's own commits run the repository's hooks as usual.

### Conflicts

```
  C  iter 1  build ✔ check ✔ review ✔ integrate ✖ conflict (merge: src/auth.rs)
  C  iter 2  build ✔ check ✔ review ✔ integrate ✔ merged
```

- `conflict` ([schema](schemas/conflict.schema.json)) carries `kind`, `integration_tip` (`T`), `reviewed_commit` (`R`), and either the conflicting `paths` or the failing `checks`.
- The next Iteration's Builder reads it as the optional Input `conflict`. Its instructions say: merge `integration_tip` into the Task Branch with `git merge`, resolve, make the Checks pass and commit. Merging rather than rebasing keeps the earlier approved commit in history, so it can be compared with the new one. The delivery check enforces that the new commit descends from `integration_tip`.
- The new commit goes through `check` and review again. It is a SHA nobody reviewed, and the verdict covers all of it. The Reviewer also reads `conflict`, and its instructions focus it on the delta since the previously approved commit ([G. State Machines](G-state-machines.md#what-the-reviewer-reads-after-the-first-iteration)).
- Inputs resolve to the latest Revision by name, so a later Iteration can still see an old `conflict` or `feedback`. The Context Assembler labels every Input with the Iteration and step that produced it and states why the current Iteration started ([F](F-agent-contract.md#instructions-and-assembled-context)).

### Ownership of the Integration Branch

The Integration Branch belongs to Surt until the run ends. A human who wants to look uses `git log`/`git diff`/`git show` on it, or a worktree of their own on a detached `HEAD`.

```
✖ integrate C: surt/20261016-add-login is checked out in /Users/you/surt
  switch away, then: surt start 20261016-add-login

✖ surt/20261016-add-login moved outside surt (expected 9e04b7d, found 71aa0c2)
  restore it: git branch -f surt/20261016-add-login 9e04b7d
```

- Checked out anywhere: the Execution fails technically and the next driving command retries. Moving a checked-out ref with `update-ref` would leave that worktree out of sync.
- Tip differs from the recorded one: Surt stops and never adopts it, because an unreviewed commit would enter. The same detection catches a Builder that moved the branch with its own Git. A driving command refuses to start while the branch is moved or checked out; a move found during a run blocks the Operation ([I](I-recovery.md#the-integration-branch)).

## Snapshot, Shipper and delivery

- `git.snapshot_run_branch` runs after `implement`. It checks ownership like `integrate` step 1, verifies that the tip descends from `base.sha`, and produces `integration` ([schema](schemas/integration.schema.json)): branch, Base, final tip, and each Task's key, reviewed commit and merge commit. A parent does not read its children's artifacts ([E](E-workflow-schema.md#per-item-steps)), so this list comes from Git and SQLite state, not from child artifacts.
- The **Shipper** is a read-only agent. It reads `manifest` and `integration`, inspects `git log`/`git diff <base>..<tip>` from the project root without checking anything out, and delivers `delivery` = `{ title, summary }` ([schema](schemas/delivery.schema.json)): text ready to become a pull request description, summarizing the spec's intent, the Tasks and the Checks. The delivered branch and SHA are those of `integration`; the agent does not declare them.
- Nothing is pushed. The spec is not committed. The Integration Branch is the deliverable:
  ```
  $ surt status 20261016-add-login
  ✔ shipped  surt/20261016-add-login @ 4d2e9a1  (base main @ a11c9d0, 3 tasks)
    surt show 20261016-add-login delivery
  ```

## Merging into the Base (opt-in)

```toml
# .surt/config.toml
[git]
merge_into_base = "never"     # default | "ff-only"
```

The value belongs to the Operation, not to the Shipper or the `ship` step, and is frozen at root run creation. `git.merge_into_base` runs after `ship`:

| Condition | Result |
|---|---|
| `merge_into_base = "never"` | `skipped` (disabled) |
| `base.ref` is `null` | `skipped` (no base branch) |
| `refs/heads/<base.ref>` ≠ `base.sha` | `skipped` (base moved) |
| Base branch checked out in a worktree with changes or an operation in progress | `skipped` (worktree not clean) |
| Base branch checked out in a clean worktree | `git -C <worktree> merge --ff-only <tip>` → `merged` |
| Base branch not checked out | `update-ref refs/heads/<base.ref> <tip> <base.sha>` under `git.lock` → `merged` |

```
✔ merged into main (fast-forward a11c9d0 → 4d2e9a1)
⚠ not merged: main moved since base (a11c9d0 → b22e310); merge surt/20261016-add-login yourself
```

It never creates a merge commit on the Base branch: a combination with commits that arrived later would be unreviewed and unchecked. `skipped` is a successful Outcome; the feature is shipped either way, and the run still ends `succeeded` (exit `0`). Rationale: [ADR 0006](../adr/0006-merge-into-base-is-opt-in-fast-forward.md).

The rows are checked in the order of the table. Each `skipped` records one reason with the step's `step.done` event ([D](D-database.md#event-data)):

| Reason | Row | Line printed after `✔ shipper delivered` |
|---|---|---|
| `disabled` | `never` | none: the default output is unchanged |
| `no_base_branch` | `base.ref` is `null` | `⚠ not merged: the base a11c9d0 is not a branch; merge surt/<subject> yourself` |
| `base_moved` | the branch is not at `base.sha`, or the compare-and-swap of `update-ref` failed; `found` is where it is (`null` when it no longer exists) | `⚠ not merged: main moved since base (a11c9d0 → b22e310); merge surt/<subject> yourself` |
| `worktree_not_clean` | the branch is checked out in a worktree that is not clean | `⚠ not merged: main is checked out in a worktree that is not clean; merge surt/<subject> yourself` |

`merged` records `from` (`base.sha`) and `to` (the delivered tip) and prints `✔ merged into main (fast-forward a11c9d0 → 4e1b2c0)`. The `update-ref` writes the reflog message `surt: fast-forward <base.ref> to surt/<subject>`, so `git reflog main` names the run that moved it. Before it moves the ref the Operation commits a `ref_move` Intention (`refs/heads/<base.ref>`, `base.sha` → tip), settled by recovery's [ref move rule](I-recovery.md#ref-moves). Surt never pushes.

**The checked-out case.** The worktree holding `refs/heads/<base.ref>` is the entry of `git worktree list --porcelain -z` whose `branch` names it. That worktree is **clean** when `git status --porcelain=v2 --untracked-files=no` lists nothing (no staged or unstaged change to a tracked file) and none of `MERGE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`, `rebase-merge` or `rebase-apply` exists under its `git rev-parse --git-path` (status does not report an operation in progress). Untracked files do not count. A clean worktree gets its `ref_move` Intention with `worktree_path` set, then `git -C <worktree> merge --ff-only <tip>` under `git.lock`, so its files and its branch move together. When Git refuses because the fast-forward would overwrite an untracked file (or a local change that appeared after the check), Git has touched nothing: the Intention is discarded and the Outcome is `skipped` (`worktree_not_clean`). Any other failure, such as a file in use on Windows, is a technical failure that keeps the Intention; the next Attempt settles it by the [ref move rule](I-recovery.md#ref-moves), which blocks with [`merge_into_base_interrupted`](I-recovery.md#an-interrupted-fast-forward-into-the-base) when the files moved and the ref did not.

**Hooks.** Surt runs `git merge` as Git runs it, so the repository's `post-merge` hook runs in that worktree after the fast-forward. Its exit status does not change the Outcome: Git ignores it too. `GIT_REFLOG_ACTION` is set to the same `surt: fast-forward <base.ref> to surt/<subject>`, so the reflog entry still names the run. The `update-ref` of the not-checked-out case runs no hook but `reference-transaction`.

## Git configuration Surt applies

| Where | What |
|---|---|
| Surt's read commands | `--no-optional-locks`, so polling never takes `index.lock` |
| Surt's commands on Windows | `-c core.longpaths=true` |
| Surt's commands with Git ≥ 2.56 | `-c core.lockfilePid=true`, which records the owner PID of a lock for diagnostics |
| Agent processes on Windows | `core.longpaths=true` injected through `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_<n>` / `GIT_CONFIG_VALUE_<n>`, appended to any the user already set |

Surt never writes the repository's `.git/config` or the user's global Git configuration. It imposes no `core.autocrlf` value.

## Decided elsewhere

- Reaction to a Builder delivering no new commit, iteration limits, the scope of a re-review after a conflict, and what a blocked child means for its dependents: [G. State Machines](G-state-machines.md).
- Reconciling each partial state of these Operations (branch without worktree, `locked initializing` worktree, ref moved outside Surt), worktree preparation before every Attempt, and an interrupted fast-forward into the Base: [I. Recovery and Reconciliation](I-recovery.md).
- What happens to worktrees and branches of terminal runs and on `surt delete`: [I. Cleanup, Delete and Logs](I-cleanup.md).
- `--base`, the per-run override `--merge`/`--no-merge`, and the shape of refusals: [J. CLI Surface](J-cli.md).
- Physical tables for worktrees, tips and the project id: [D. Database Schema](D-database.md#git-state).

Decision records: [ADR 0006 — Merging into the Base is opt-in and fast-forward only](../adr/0006-merge-into-base-is-opt-in-fast-forward.md), [ADR 0007 — Tasks are integrated as merge commits of the reviewed SHA](../adr/0007-integrate-reviewed-sha-as-merge-commit.md).
