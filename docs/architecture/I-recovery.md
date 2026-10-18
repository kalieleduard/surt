# I. Recovery and Reconciliation

How Surt resumes any Workflow Run, days later, after a crash, a reboot, a `kill -9` or a second Ctrl-C, without duplicating work and without trusting an old Claude session. Vocabulary (**Recovery**, **Intention**, **Salvage Ref**, **Transient Failure**, **Driver**, **Interruption**) is defined in [`CONTEXT.md`](../../CONTEXT.md). Decision ticket: [Recovery and reconciliation strategy](../../.scratch/surt-architecture/issues/12-recovery-and-reconciliation.md). Facts about Git's partial states come from [Git worktree/merge mechanics and partial states](../../.scratch/surt-architecture/issues/04-git-worktree-merge-partial-states.md) (full table on branch `research/git-worktree-merge`).

## Principles

1. **Recovery is not a mode.** There is no `surt doctor`, `repair` or `--recover`. Every driving command (`surt feature`, `plan`, `start`) runs the same Recovery right after it takes the run lock, whether the last Driver stopped cleanly or not. Every Operation is written as *read state → compare with SQLite → act*, so its next Attempt is its own reconciliation.
2. **Only what the Driver recorded finishing is accepted.** An Execution whose end the Driver did not commit is `interrupted`, even if its process exited 0 and left a valid `delivery/`. Without the stream there is no exit code and no `result` event, and accepting it would be adoption ([A. Process Model](A-process-model.md#orphaned-agents)).
3. **Intention → effect → confirmation.** Before any effect outside SQLite whose result recovery must recognize, Surt commits an **Intention** in its own transaction. The effect follows. A second transaction confirms it. `synchronous = FULL` ([D](D-database.md#opening-the-database)) guarantees a committed Intention survives a power cut. Recovery only reads Intentions and compares; it never recomputes what an effect should have been.
4. **Surt repairs only what it owns.** Task worktrees under `~/.surt/worktrees/`, `refs/heads/surt/…` and `refs/surt/…` are Surt's, and it fixes them by itself. The user's worktrees, the Base branch and any other ref or lock are never written during recovery. Problems there become a blocked step or a refusal with exact instructions.

## What a driving command does when it takes the lock

```
1. preflight        git ≥ 2.38, identity, project root            (A. Git Integration)
2. try_lock         .surt/runtime/locks/<root-run-id>.lock          (A. Process Model)
3. stale work       queued/running Executions → stop survivors → interrupted
4. intentions       ref_move rows: run the owning Step Run's next Attempt now
                    Working Copy scan, pending baselines first
5. branch check     the root run's branch is at tip_sha and not checked out, or refuse
6. unblock          blocked → running with a fresh budget                  (G)
7. schedule         every Attempt in a worktree prepares it first; agent launches
                    pause while a Transient Failure waits
```

```
$ surt start 20261017-add-login
⚠ previous driver (pid 48213) is gone — recovering
  ⚠ builder task B (pid 50122, alive) — stopping it
  ✖ builder task C exited but was never recorded — discarded
  ✔ integrate task A had completed (surt/20261017-add-login @ 3c1aa20) — recorded
↻ task B build #1: attempt 2
  ⚠ worktree had leftovers from attempt 1 — saved to refs/surt/salvage/0192f3…
↻ task C build #1: attempt 2
```

Steps 3 and 4 always run to completion and record what they find, even when step 5 then refuses. They only make SQLite tell the truth about what already happened.

## Stale Executions and orphans

The lock was free, so every `queued` or `running` Execution of the root run and its children is stale by definition ([A](A-process-model.md#ownership-one-driver-per-root-run)).

- `queued` → `interrupted`.
- `running` with a recorded process identity: if a process with the same pid, native start time and boot id still exists, Surt stops its group with the usual escalation (graceful, `stop_grace_period`, forced) and records `execution.orphan_stopped`. A different boot id means the machine rebooted and the process is gone. Then → `interrupted`.
- `running` with a recorded identity whose process is gone → `interrupted`, reported as `✖ <step> attempt N exited but was never recorded — discarded`.
- `running` with no recorded identity (the residual window below, or a runtime that runs inside the Driver) → `interrupted`, reported as `⚠ <step> attempt N was interrupted with the previous driver — discarded`. Nothing says the process exited on its own.
- Operations that spawn a long-running process tree record its identity in the same columns while it runs: each Check of `checks.run`, and the Checks of `git.integrate`'s re-check. An orphaned Check is stopped like an orphaned agent.
- `interrupted` never counts toward `max_attempts`. Its Step Run stays `running` with no live Execution, and the scheduler starts its next Attempt ([G](G-state-machines.md#execution)).

**Residual window:** the identity is recorded right after the spawn, in its own transaction. A crash in the milliseconds between the two leaves a process Surt cannot identify. On Linux (`PR_SET_PDEATHSIG`) and Windows (Job Object with kill-on-close) it dies with the Driver. On macOS it can survive. It cannot harm the result: the worktree is prepared again before the next Attempt, and nothing it commits is accepted, because acceptance binds to recorded SHAs. At worst it leaves noise for the next preparation to salvage.

## Intentions

| Effect | Intention, committed before | Confirmation, committed after |
|---|---|---|
| create a run's branch (and worktree) | the `run_branch` row itself: `ref_name`, `start_sha` = `tip_sha` = target SHA, `worktree_path` | the Step Run's Outcome |
| move a ref (`git.integrate`, `git.merge_into_base`) | a `ref_move` row: `ref_name`, `old_sha`, `new_sha`, `worktree_path` when the move is a checkout | the Operation's result, and the row deleted, in one transaction |
| rewrite a Working Copy file | `working_copy_file.pending_sha256` / `pending_revision_id`, in the publication transaction | the pending values promoted to the baseline |
| launch a process | the `execution` row (`running`), then its identity | the Execution's final state |

### Ref moves

One rule for every `ref_move` row, applied by the next Attempt of the Step Run that owns it. Recovery step 4 runs that Attempt before the branch check and before anything else is scheduled:

| Ref now equals | Meaning | Action |
|---|---|---|
| `new_sha` | the move happened | confirm: record the Operation's result (`git.integrate`: `tip_sha` = M, the child's `merged_as_sha` = M, Outcome `merged`), delete the row |
| `old_sha` | the move did not happen | delete the row and redo the Operation from its start; a move with `worktree_path` whose worktree is not clean instead **keeps** its row and blocks ([below](#an-interrupted-fast-forward-into-the-base)) |
| anything else | someone else moved it | delete the row; `git.integrate` → `blocked` (`not_retryable`, `integration_branch_moved`); `git.merge_into_base` → its normal `skipped` (base moved) |

A `ref_move` row exists only while its Step Run is `running`, or `blocked` on an interrupted fast-forward. Before it moves the ref, `git.integrate` still re-checks the merge result when the tip moved ([A. Git Integration](A-git-integration.md#integration)). A ref equal to `new_sha` therefore always passed the Checks, and confirmation never runs them again. The merge commit's dates still come from `step_run.started_at`, so a redone integration recomputes the same `M`. That is a second guarantee, not something recovery relies on.

`status` reads the same rows:

```
  B  ⟳ integrating   surt/20261017-add-login: 9e04b7d → 3c1aa20 (pending)
```

### Working Copy files

Publishing a Revision of an artifact with a Working Copy sets `pending_sha256` and `pending_revision_id` on each of its existing `working_copy_file` rows, in the publication transaction. The file is then written to a temporary file and renamed over the old one. A second transaction promotes the pending values to `sha256`/`revision_id`. A file new to the Working Copy gets its row with the new baseline directly. If the crash comes before the write, the file is missing, and a missing file is recreated, never an edit ([H](H-artifact-storage.md#edit-detection)).

Edit detection handles pending rows first:

| The file's hash | Action |
|---|---|
| = `pending_sha256` | the write happened: promote |
| = the old `sha256`, or the file is missing | the write did not happen: rewrite it from the pending Revision, then promote |
| anything else | a human edited in the crash window: promote, then record the file as a human-origin Revision **after** the agent's, with a warning |

Without this, a crash between publication and the write would leave the old content under a new baseline, and the next scan would record a false human edit.

## Operations

What each Operation's next Attempt does. Every one of them is safe to run again.

| Operation | Intention | Next Attempt |
|---|---|---|
| `git.create_run_branch` | root `run_branch` row (`start_sha` = `tip_sha` = `base.sha`) | ref absent → create it · at `base.sha` → `done` · elsewhere → `blocked` (`not_retryable`) |
| `git.create_child_worktree` | child `run_branch` row (`start_sha` = `tip_sha` = the root's `tip_sha`, `worktree_path`) | the [worktree states](#worktree-states) table; a branch at another SHA falls into the salvage of [Preparing a worktree](#preparing-a-worktree-for-every-attempt) |
| `checks.run` | none: its only effect is in the worktree, which is prepared again | runs every Check again |
| `git.integrate` | `ref_move` (T → M) | [ref move rule](#ref-moves); a `checkout --detach M` left by an interrupted re-check is undone by worktree preparation |
| `git.snapshot_run_branch` | none: it only reads | runs again |
| `git.merge_into_base` | `ref_move` (`base.sha` → tip, `worktree_path` when the Base branch is checked out) | [ref move rule](#ref-moves) plus [the interrupted fast-forward case](#an-interrupted-fast-forward-into-the-base) |

Every worktree is created with `git worktree add --lock --reason "surt <run-id>"`, so `git gc` never prunes the metadata of a worktree whose run resumes months later. The cleanup unlocks it before removing it ([I. Cleanup](I-cleanup.md#cleanup-steps)).

## Preparing a worktree for every Attempt

Before **every** Execution whose working directory is a run's worktree (agent or Operation, first Attempt or not, after a crash or after a plain `failed`), Surt brings the worktree to one known state. There is no separate recovery path: the normal flow and recovery run the same code.

**Target:** the run's branch checked out at `run_branch.tip_sha`. That is the last tip Surt accepted: `start_sha` until a delivery is accepted, then the commit of the latest accepted Execution. Accepting an Execution that ran in a worktree records the branch's tip as `tip_sha` in the publication transaction. For the Task Cycle, `build` starts from the previous Iteration's commit, and `check`, `review` and `integrate`'s re-check all see the commit under review. Rationale: [ADR 0010](../adr/0010-every-attempt-starts-from-a-clean-worktree.md).

Steps, under `git.lock` where they touch shared refs, all with `LC_ALL=C`:

1. **Locate.** Read `git worktree list --porcelain -z` and apply [Worktree states](#worktree-states).
2. **Locks.** No Execution of this run is live and the Driver holds the run lock, so nothing owns the worktree's `index.lock` or other per-worktree locks: remove them. A stale `<ref>.lock` of the run's branch is removed under `git.lock` ([Git lock files](#git-lock-files)).
3. **Salvage.** If anything differs from the target (changes or untracked files that are not ignored, `HEAD` or the branch off `tip_sha`, any marker of a merge, rebase, cherry-pick, revert, `am` or bisect), Surt records everything in a commit and points the **Salvage Ref** `refs/surt/salvage/<execution-id>` at it. `<execution-id>` is the run's last Execution. The commit is built with a temporary index (`GIT_INDEX_FILE`): `read-tree HEAD`, `add -A`, `write-tree`, `commit-tree` with `HEAD` and the branch tip as parents. It works even with an unmerged index and conflict markers, and it touches neither the real index nor any branch.
4. **Clear operations in progress.** `rebase --quit`, `cherry-pick --quit`, `revert --quit`, `am --quit`, `bisect reset`, but only where their marker exists, because these commands fail without one.
5. **Reset.** `git checkout --force -B <branch> <tip_sha>`, then `git clean -fd`. There is no `-x`: ignored files, such as a warm build cache, survive.
6. **Verify.** `HEAD` is the branch at `tip_sha`, `status --porcelain=v2 --untracked-files=all` is empty, no marker remains. Otherwise the Execution fails as a launch error (`retryable`), or `not_retryable` when the worktree path belongs to someone else.

Commits a human made on a Task Branch are salvaged and reset like any other leftover: the Task Branch belongs to Surt while its run is active, like the Integration Branch.

**When it speaks.** The Salvage Ref and a `worktree.salvaged` event are always written; objects are shared, so this is cheap. The terminal shows a warning only when the leftovers were unexpected: the previous Execution in that worktree ended `failed` or `interrupted`, or `HEAD`/the branch was not at the target. Untracked output that a `succeeded` Check left behind is cleaned silently.

```
↻ task B build #2: attempt 2
  ⚠ worktree had leftovers from attempt 1 — saved to refs/surt/salvage/0192f3…
    inspect: git log -p refs/surt/salvage/0192f3…
```

Salvage Refs live until the cleanup of a `succeeded` run; a `cancelled` run keeps them, and salvages its worktrees once more before removing them, until `surt delete` ([I. Cleanup](I-cleanup.md#what-each-ending-removes)).

### Worktree states

| Found | Automatic action |
|---|---|
| branch exists, no worktree (Ctrl-C during `add -b`) | `worktree add --lock <path> <branch>` without `-b`; then preparation salvages and resets if the branch is not at `tip_sha` |
| `locked initializing` (hard kill during `add`) | `worktree remove -f -f <path>`, then add again |
| directory deleted (`prunable`) | `worktree prune` for that entry only, then add again at the recorded `worktree_path` |
| main repository moved (the worktree's `.git` points at the old place) | `git worktree repair <worktree_path>` (idempotent) |
| `.git/worktrees/<id>` deleted, directory intact | rename the directory to `<path>.orphaned-<timestamp>`, add again, warn |
| `worktree_path` exists but is not a worktree of this repository | `blocked` (`not_retryable`, `worktree_path_foreign`): Surt never deletes what it did not create |

```
↻ task B check #3: attempt 2
  ⚠ worktree for task B was missing — recreated at ~/.surt/worktrees/surt-3fa91c/20261017-add-login/B
```

Uncommitted work in a deleted or orphaned worktree is gone or set aside. It was never accepted, because acceptance binds to committed, recorded SHAs.

### Git lock files

Git never removes a stale lock. Surt removes one only when the lock is **provably its own** and its owner is dead:

| Lock | Removed by Surt when |
|---|---|
| `index.lock` and other per-worktree locks in a run's worktree | every Execution of that run is final (step 2 above) |
| `<ref>.lock` of `refs/heads/surt/…` or `refs/surt/…` | Surt holds `git.lock` and no Execution of the run owning that ref is live |
| anything else: the user's `index.lock`, `packed-refs.lock`, `HEAD.lock`, other refs | **never** |

For a lock it does not own, the Operation retries with backoff ([A. Process Model](A-process-model.md#native-git-operations-across-processes)). After that, the Execution fails (`retryable`) with a message naming the file:

```
✖ integrate C attempt 1: git ref lock held: .git/packed-refs.lock (since 14:02)
  if no git process is running, remove it, then: surt start 20261017-add-login
```

## The Integration Branch

**At the start of a driving command** (step 5), once Recovery has settled every `ref_move`, the root run's branch must be at `tip_sha` and checked out in no worktree. Otherwise the command refuses. It changes no Step Run and launches no agent, and `status` keeps showing `INTERRUPTED`:

```
✖ surt/20261017-add-login moved outside surt (expected 9e04b7d, found 71aa0c2)
  restore it: git branch -f surt/20261017-add-login 9e04b7d
  then: surt start 20261017-add-login

✖ surt/20261017-add-login is checked out in /Users/you/surt
  switch away, then: surt start 20261017-add-login
```

**During a run,** a move cannot be refused before it happens. The Operation that finds it (`git.create_child_worktree` reading the start tip, `git.integrate` step 1, `git.snapshot_run_branch`) blocks on the first occurrence (`not_retryable`, `integration_branch_moved` or `integration_branch_checked_out`, [G](G-state-machines.md#attempts)). The next driving command first refuses at step 5 until the branch is restored. Once it is, the command unblocks the step, and the new Attempt confirms the restoration by finding ref = `tip_sha`. No column records that a move was seen: the comparison is the record.

## An interrupted fast-forward into the Base

With `merge_into_base = "ff-only"` and the Base branch checked out in the user's worktree, `git merge --ff-only` updates the files before the ref. A second Ctrl-C in between leaves files partly updated and the ref at `base.sha`. The `ref_move` row has `worktree_path` set. If its next Attempt finds the ref at `old_sha` and that worktree not clean, the Step Run becomes `blocked` (`not_retryable`, `merge_into_base_interrupted`). Surt never finishes or undoes the checkout itself, because the worktree may hold unrelated changes of the user's:

```
⛔ merge into main was interrupted; /Users/you/surt may be partially updated
   finish it:  git -C /Users/you/surt merge --ff-only 4d2e9a1
   or undo it: git -C /Users/you/surt reset --hard a11c9d0   (discards changes)
   then: surt start 20261017-add-login
```

The row stays, so the next command's Attempt applies the same rule. After *finish it* the ref equals `new_sha`, so the Outcome is `merged`. After *undo it* the ref equals `old_sha` and the worktree is clean, so the fast-forward runs again. If nothing was done, the step blocks again.

## Transient Failures: network and rate limits

Claude Code already retries API errors by itself (`system/api_retry`). When it gives up, Surt classifies the failure from the stream (`assistant.error`, the `result`'s `terminal_reason`) instead of spending an Attempt on every kind:

| Class | Errors | Effect |
|---|---|---|
| `transient` | `rate_limit`, `overloaded`, `server_error`, transport or network errors | Execution `failed` with `failure_class = 'transient'`; **does not count** toward `max_attempts`; agent launches pause |
| `not_retryable` | `authentication_failed`, `billing_error`, `model_not_found`, `invalid_request`, and the launch errors of [G](G-state-machines.md#attempts) | Step Run `blocked` at once |
| `retryable` | every other technical failure ([F](F-agent-contract.md#runtime-success-and-failure)) | counts; a new Attempt at once |

Observed in the real streams of [Capture a real Claude Code stream as a fixture](../../.scratch/surt-architecture/issues/19-real-claude-code-stream-fixture.md): `authentication_failed` and `model_not_found` arrive in `assistant.error` with a `result` of `terminal_reason = "api_error"` (`api_error_status` 404 for the model); a rate-limited or network failure was not captured. An unknown error category is `retryable`, so a misclassification costs Attempts and never makes Surt wait forever on a permanent error.

**The pause.** An account rate limit hits every agent at once, so a Transient Failure pauses **every agent launch of that Driver**, not only its own Step Run:

- Until the reset time when the stream reports one (`rate_limit_event`; it also arrives on normal runs with `rate_limit_info.status = "allowed"`, which is not a limit), otherwise with backoff: 1, 2, 4, 8 and then 15 minutes between tries.
- Executions already running go on. If they hit the limit, they fail `transient` too.
- While paused, the Driver takes no new agent slot. Operations keep running: a Check or an integration does not need the API.
- The backoff resets when an agent Execution ends in any other way.
- There is no ceiling. The command is in the foreground, nothing is spent while it waits, and Ctrl-C turns the wait into an ordinary Interruption.

```
✖ task B build attempt 2: rate limited (resets 15:00)
⏸ agent launches paused until 15:00 — 3 waiting · Ctrl-C to stop (resumable)
```

The pause lives in the Driver's memory. A new driving command starts unpaused and learns again from its first failure. Several Drivers (other Features, other projects) do not coordinate their pauses; account-wide pressure across Drivers stays open on the map.

Operations never fail `transient`. Git lock contention from third parties is retried with backoff inside the Operation ([A](A-process-model.md#native-git-operations-across-processes)).

## Transactional checkpoints

Every transition below is one `BEGIN IMMEDIATE` transaction with its `event`. Between two checkpoints, an external effect happens at most once. Each one's Intention or reread makes it safe to find half done:

| Checkpoint | Commits together |
|---|---|
| run created | Subject, root run, snapshots, Initial Artifacts, top-level Step Runs `pending` ([D](D-database.md#when-rows-are-created)) |
| Iteration started / Per-Item expansion | the new Step Runs or Child Workflow Runs, pins |
| Execution started | `execution` `running` (or `queued`), its `execution_input` rows |
| process spawned | the process identity (`pid`, start time, boot id) |
| Intention | `run_branch` row, or `ref_move` row |
| Execution accepted | Revisions, files and blobs, Outcome, Step Run transition, `tip_sha`, Working Copy pending values, `ref_move` deleted |
| Working Copy written | pending values promoted to the baseline |
| Execution ended otherwise | `failed`, `interrupted` or `cancelled`, with class and error; `blocked` when a limit or a `not_retryable` failure applies |

## Without a Driver: `surt cancel` and `surt status`

**`surt cancel`** with the lock free takes it and runs steps 3 and 4 before recording the Cancellation: it stops surviving orphans and settles every `ref_move` against the ref. For a cancelled run, settling records the Git fact (`tip_sha`, `merged_as_sha`) and deletes the row; the Step Run is then cancelled with the rest. The history of a cancelled run therefore says where its branch really is. It does not run steps 5 to 7. If a fast-forward into the Base was interrupted, cancel proceeds and prints the same instructions for the user's worktree.

```
$ surt cancel 20261017-add-login
  ⚠ builder task B (pid 50122, alive) — stopping it
  ✔ integrate C had completed (surt/20261017-add-login @ 3c1aa20) — recorded
✔ run cancelled
```

**`surt status`** never takes the run lock and never recovers. It only reads; even a Working Copy edit is shown, not recorded ([J](J-cli.md#reading-never-writes)). When no Driver holds the lock and a `running` Execution's recorded process still exists, it says so, so nobody mistakes an agent editing a worktree unattended for a stopped run:

```
20261017-add-login   INTERRUPTED   (no driver)
  B  ⚠ builder still running unattended (pid 50122) — stopped on next: surt start 20261017-add-login
```

## Decided elsewhere

- Ownership by file locks, orphan stopping, the first and second Ctrl-C: [A. Process Model](A-process-model.md).
- Execution and Step Run states, Attempt and Iteration limits, leaving `blocked`: [G. State Machines](G-state-machines.md).
- Columns of `ref_move`, `run_branch`, `working_copy_file` and `execution.failure_class`: [D. Database Schema](D-database.md) and [`migrations/0001_initial.sql`](migrations/0001_initial.sql).
- What the cleanup and `surt delete` remove (Salvage Refs, worktree locks, branches, logs), and the retry of a pending cleanup by the next command that takes the lock: [I. Cleanup, Delete and Logs](I-cleanup.md).
- Message shape and exit codes of the refusals and warnings above: [J. CLI Surface](J-cli.md) (a refusal at start exits `3`, `merge_into_base_interrupted` exits `4`).

Decision record: [ADR 0010 — Every Attempt starts from a clean worktree; leftovers are salvaged, never continued](../adr/0010-every-attempt-starts-from-a-clean-worktree.md).
