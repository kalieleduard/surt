# I. Cleanup, Delete and Logs

What Surt removes when a Workflow Run ends and when the user deletes a Feature, what it keeps, in which order, and where agent logs live. Companion to [I. Recovery and Reconciliation](I-recovery.md). Vocabulary (**Cleanup**, **Purge**, **Salvage Ref**, **Working Copy**, **Integration Branch**, **Task Branch**) is defined in [`CONTEXT.md`](../../CONTEXT.md). Decision ticket: [Cleanup, `surt delete` and logs](../../.scratch/surt-architecture/issues/13-cleanup-delete-and-logs.md).

## Principles

1. **Cleanup follows a terminal state; it is not a Step.** It appears in no Workflow Definition and has no Step Run. The only per-workflow control is `[cleanup] on_success = "clean" | "keep"` ([E](E-workflow-schema.md)).
2. **Every item is idempotent, and a failed cleanup is retried, not fatal.** Something that no longer exists counts as removed. A failure leaves the cleanup pending and never changes the command's exit code: the run's result is what the exit code reports.
3. **Cleanup removes only views and scratch space.** The Working Copy, Execution input/delivery directories and Task worktrees can be regenerated or were never accepted. History (SQLite rows, logs) and work that was not integrated (refs of a cancelled run) leave only with `surt delete`.
4. **Nothing a human typed is removed by cleanup.** A Working Copy file with an unrecorded edit is kept.
5. **Surt removes only what it created.** The Base, the user's worktrees and any path that is not a worktree Surt recorded are never touched, by cleanup or by `delete` ([I. Recovery](I-recovery.md#principles), principle 4).

## When cleanup runs

The process that records a root run's terminal state runs the cleanup right after that transaction, still holding the run lock:

- `succeeded`: the Driver, after the Shipper's delivery and the opt-in fast-forward.
- `cancelled`: the Driver that applies a cancel request, or `surt cancel` itself when no Driver is live ([I](I-recovery.md#without-a-driver-surt-cancel-and-surt-status)).

```
✔ shipper    delivered — surt/20261017-add-login @ 4e1b2c0
✔ run        succeeded
↻ cleanup    3 worktrees · 3 task branches · 2 salvage refs · working copy
```

**State.** The root run's `cleaned_at` ([D](D-database.md#workflow-runs)) is written in the transaction that ends the cleanup. A root run is **cleanup pending** when it is terminal, its policy calls for a cleanup (`cancelled`, or `succeeded` with `on_success = "clean"`), and `cleaned_at` is NULL. A `succeeded` run with `on_success = "keep"` is **kept**: `cleaned_at` stays NULL and nothing is pending.

**Failure.** One item failing (on Windows, an antivirus or an editor holding a file in a worktree) does not stop the others, except that a Task Branch whose worktree could not be removed is also kept, because it is still checked out there. `cleaned_at` stays NULL, a `cleanup.incomplete` event lists what is left, and the command exits as the run's result says:

```
✔ run        succeeded
⚠ cleanup incomplete — could not remove ~/.surt/worktrees/surt-3fa91c/20261017-add-login/B (file in use)
    retry: surt start 20261017-add-login
```

**Retry.** Every command that takes the run lock of a cleanup-pending run finishes the cleanup before anything else: `surt feature`, `plan`, `start` and `cancel` on that Feature (on a terminal run they have nothing else to do) and `surt delete`. A driving command never cleans other Subjects' runs, whose locks it does not hold. Reading commands never clean ([J](J-cli.md#reading-never-writes)).

**Final warnings.** Three situations end the cleanup with a warning instead of leaving it pending, because retrying would not change them: a Working Copy file with an unrecorded edit ([below](#unrecorded-working-copy-edits)), a Task Branch that someone moved away from its recorded `tip_sha`, and a recorded `worktree_path` that is no longer a worktree of this repository. `cleaned_at` is written and each case is listed in the `cleanup.finished` event.

## What each ending removes

| Thing | `succeeded`, `clean` | `succeeded`, `keep` | `cancelled` | `surt delete` |
|---|---|---|---|---|
| Working Copy `.surt/features/<subject>/`, files equal to their baseline | remove | keep | remove | remove |
| Working Copy files with an unrecorded edit | keep, warn | keep | keep, warn | remove |
| `.surt/runtime/executions/<id>/` (`inputs/`, `delivery/`) | remove | keep | remove | remove |
| Logs `.surt/logs/<subject>/` | keep | keep | keep | remove |
| Task worktrees | remove | keep | salvage, then remove | remove, no salvage |
| Task Branches `surt/task/<subject>/<key>` | delete if at `tip_sha` | keep | keep | delete |
| Salvage Refs of the run | delete | keep | keep | delete |
| Integration Branch `surt/<subject>` | keep | keep | keep | keep if the run `succeeded`, delete if `cancelled` |
| SQLite rows | keep | keep | keep | purge |
| Run lock file `.surt/runtime/locks/<root-run-id>.lock` | keep | keep | keep | remove after the purge |

- **Succeeded.** The Task Branches' reviewed commits stay reachable from the Integration Branch through its merge commits ([A](A-git-integration.md#integration)), so deleting the branches loses nothing that was accepted. Salvage Refs held leftovers that were never accepted.
- **Cancelled.** Work that was never integrated (a Task in its third Iteration, the leftovers of a Builder stopped by the cancel) survives as refs, which are cheap. The worktrees, which are not cheap, go. The `on_success` policy does not apply to a cancelled run.
- **Integration Branch.** It is the product of a `succeeded` run and may be the source of an open pull request, so even `surt delete` keeps it. The Integration Branch of a `cancelled` run is Surt's unfinished work and leaves with the history.
- **SQLite.** After cleanup, `status`, `inspect`, `show` and `logs` keep working: Revisions are blobs in SQLite ([H](H-artifact-storage.md#after-a-run-ends)) and logs stay on disk.

### Cleanup steps

In this order, each one skipped when its effect is already there:

1. **Working Copy.** Remove each file whose SHA-256 equals its baseline; keep and warn about the others; remove the directories left empty.
2. **Execution directories.** `remove_dir_all` of `.surt/runtime/executions/<id>/` for every Execution of the root run and its Child Workflow Runs.
3. **Worktrees**, one per Child Workflow Run with a `run_branch.worktree_path`, under `git.lock`:
   - Locate it with `git worktree list --porcelain -z` and the [worktree states](I-recovery.md#worktree-states) of section I. A missing directory is pruned for that entry only; a path that is not a worktree of this repository is left alone with a warning.
   - **Cancelled run only:** salvage the leftovers as in [Preparing a worktree](I-recovery.md#preparing-a-worktree-for-every-attempt), step 3, into `refs/surt/salvage/<execution-id>` of the Task's last Execution. Nothing is reset.
   - `git worktree unlock <path>`, then `git worktree remove --force <path>`. Ignored files, such as a build cache, go with the directory. Untracked leftovers of a `succeeded` run were never accepted ([ADR 0010](../adr/0010-every-attempt-starts-from-a-clean-worktree.md)).
4. **Refs** (`succeeded`, `clean` only), under `git.lock`: read the run's Task Branches and Salvage Refs with `git for-each-ref`. Each Task Branch still at its recorded `tip_sha` is deleted with the expected old value; the others are kept with a warning. One `git update-ref --stdin` transaction deletes them all. A failed transaction (someone moved a ref in between) leaves the cleanup pending.
5. **Record.** One transaction writes `cleaned_at` and a `cleanup.finished` event that lists what was kept and why.

`run_branch.worktree_path` keeps its value after the worktree is removed. It is history: `inspect` shows where an Execution ran.

## Unrecorded Working Copy edits

A human edit that was never recorded can sit in the Working Copy when a run ends, for example an edit to a pinned spec, which is refused rather than recorded ([ADR 0005](../adr/0005-edits-to-pinned-revisions-are-refused.md)). Cleanup keeps it in place:

```
✔ run        succeeded
↻ cleanup    done
⚠ kept .surt/features/20261017-add-login/spec.md — unrecorded edit
```

Recording it as a human Revision is not possible for a pinned Revision, and doing it during a cancel would rewrite the history of a run that is ending.

A new run of the same Subject would write its own Revisions over that file. `surt feature <feature> --new-run` therefore refuses while the Subject's Working Copy holds a file that differs from the previous run's baseline:

```
$ surt feature 20261017-add-login --new-run
✖ working copy has unrecorded edits from run 1
    .surt/features/20261017-add-login/spec.md
    fix: move or delete the file, then run again
```

It exits `3` ([J](J-cli.md#exit-codes)). `surt delete` removes those files and lists them in its confirmation.

`surt restore` never rewrites the Working Copy of a run that is over. On a terminal run it refuses with exit `3` and the same way out:

```
$ surt restore 20261017-add-login spec
✖ 20261017-add-login run 1 has ended; restore only rewrites an active run
    .surt/features/20261017-add-login/spec.md
  fix: move or delete the file
```

## Logs

Logs are history, not scratch space. They live outside `runtime/`, grouped by Subject:

```
.surt/logs/<subject>/<execution-id>/
├── stream.jsonl   agent: the runtime's stdout, one line per event, as received
├── stderr.log     agent: the runtime's stderr
└── output.log     Operation that spawns processes (checks.run, git.integrate's re-check):
                   combined output of each Check, with one header line per Check
```

- Files are created when the Execution starts and appended line by line, so `surt logs -f` can follow them ([J](J-cli.md#surt-logs)). Git commands an Operation runs write no log file; their diagnosis is the Execution's `error_message`.
- One directory per Subject holds the logs of every run of that Subject. `.surt/logs/` is ignored by Git like the rest of `.surt/` ([H](H-artifact-storage.md#what-is-versioned-in-the-users-git)).
- Cleanup never removes logs; `surt delete` removes the Subject's directory. There is no rotation, size limit or redaction in v1.
- `.surt/runtime/executions/<id>/` holds only `inputs/` and `delivery/`, which can be rebuilt from blobs.

**Without the log.** When an Execution's log directory is missing (removed by hand), `logs` says so and points at `inspect`. For a single Execution it refuses (exit `3`); among several sources it warns on stderr and prints the others. A missing file inside an existing directory reads as empty:

```
$ surt logs 0192f3a8
✖ no log for execution 0192f3a8-7b1c-7000-8000-4a1f0c3e9b21 (.surt/logs/20261017-add-login/0192f3a8-7b1c-7000-8000-4a1f0c3e9b21/ is missing)
  inspect: surt inspect 0192f3a8-7b1c-7000-8000-4a1f0c3e9b21
```

`inspect` needs no stream. It reads the Execution row: state, Outcome, `error_code`/`error_message`, timestamps, process identity, and `runtime_info` (Claude Code version, session id, model, effort, exit code, turn count, usage, cost, permission denials; [D](D-database.md#executions)). The Check summary is in the `check-report` artifact ([A](A-git-integration.md#checks)).

## `surt delete`

`surt delete <feature>` removes a Subject: every run's disk state, Git refs (except delivered Integration Branches) and history. It is a **purge** ([D](D-database.md#deleting-history), [ADR 0009](../adr/0009-history-is-purged-not-soft-deleted.md)).

```
$ surt delete add-login
delete feature 20261017-add-login and all its history?
  runs      1 cancelled · 2 succeeded
  git       delete surt/20261017-add-login (run 1, cancelled)
            delete surt/task/20261017-add-login/{A,B} · 1 salvage ref
            keep   surt/20261017-add-login.2 (run 2, delivered)
  disk      .surt/logs/20261017-add-login (14 MB) · 1 unrecorded edit in the working copy
  database  3 spec revisions · 41 executions · 312 events
[y/N]
```

- **Active run.** Refuses with exit `3` and `fix: surt cancel 20261017-add-login`. `delete` never stops agents.
- **Confirmation** as in [J](J-cli.md#confirmations): `--yes` skips it; without a TTY and without `--yes`, exit `3`.
- Task Branches and Salvage Refs are deleted at whatever SHA they point to, without the expected old value: the confirmation listed them.

### Order

1. **Locks.** `try_lock` on the run lock of every root run of the Subject. A busy lock (for example `surt start` finishing a pending cleanup) refuses with exit `3`.
2. **Git and disk**, each step idempotent:
   - worktrees: `unlock`, `remove --force`; a missing directory is pruned for that entry only;
   - refs: one `git update-ref --stdin` transaction;
   - directories: `.surt/logs/<subject>/`, `.surt/features/<subject>/` and every `.surt/runtime/executions/<id>/` of the Subject.
   A failure (a file in use) does the rest of this step, skips step 3 and exits `1` with `retry: surt delete 20261017-add-login`.
3. **SQLite**, in one `BEGIN IMMEDIATE` transaction: read the Subject's root runs again. If the set differs from the one step 2 cleaned (a concurrent `--new-run`), roll back and refuse with exit `3`: `feature changed while deleting; run again`. Otherwise purge: `DELETE FROM subject`, then the snapshot and blob garbage collection.
4. **Lock files.** Release the run locks and remove their files, best effort. A lock file whose run no longer exists is inert.

**Interrupted delete.** Before step 3, SQLite still holds everything, so `surt delete` again shows the remaining impact and finishes. Step 3 is atomic. There is no `deleting` marker: in between, `status` shows the Feature as it is, with `worktree missing` where it applies.

## In `surt status`

```
$ surt status --all
FEATURE                 RUN  STATE              DRIVER  PROGRESS
20261017-add-login      1    SUCCEEDED          —       4/4 merged · cleanup pending
20261016-fix-cache      1    SUCCEEDED          —       3/3 merged · kept
20261015-refactor-db    1    CANCELLED          —       2/4 merged

$ surt status add-login
20261017-add-login · run 1   SUCCEEDED
  …
  cleanup  pending — could not remove ~/.surt/worktrees/surt-3fa91c/20261017-add-login/B
           retry: surt start 20261017-add-login
```

A kept run shows `cleanup  kept (on_success = "keep") — remove with: surt delete 20261016-fix-cache`.

## Not in v1

- Removing a Task's worktree as soon as the Task is integrated. There is one cleanup, at the end of the root run.
- Redacting secrets from logs, opt-in full logs, log rotation or retention limits, archiving without deleting.
- Removing `.orphaned-*` directories ([Worktree states](I-recovery.md#worktree-states)); unreferenced commits are left to `git gc`.
- Protection against junctions and `\\?\` paths beyond what `remove_dir_all` already gives ([Cross-platform primitives (macOS, Linux, Windows)](../../.scratch/surt-architecture/issues/18-cross-platform-primitives.md)).

## Decided elsewhere

- Terminal states and when a run reaches them: [G. State Machines](G-state-machines.md).
- `cleaned_at`, the cascade of the purge and the event kinds: [D. Database Schema](D-database.md) and [`migrations/0001_initial.sql`](migrations/0001_initial.sql).
- Worktree states, salvage and the run lock taken by driving commands: [I. Recovery and Reconciliation](I-recovery.md).
- Branch names, worktree paths and how a Task is integrated: [A. Git Integration](A-git-integration.md).
- Message shape, confirmations and exit codes: [J. CLI Surface](J-cli.md).
