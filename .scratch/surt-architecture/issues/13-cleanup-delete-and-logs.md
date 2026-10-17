# Cleanup, `surt delete` and logs

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Blocked by: 08, 12, 18
Assignee: Kaliel

## Question

When a run ends (or the user deletes the Feature), what does Surt remove, what does it keep, and in what order — so that running it again after a crash finishes the job?

Decide, for v1 only:
- **`on_success = "clean"`**: what happens to each thing a run leaves behind — Working Copy (`.surt/features/<subject>/`), `.surt/runtime/executions/<id>/`, Task worktrees, Task Branches, Salvage Refs. The Integration Branch is the deliverable and stays.
- **`cancelled` run**: same treatment as `succeeded`, or keep everything for inspection?
- **`surt delete <feature>`**: Subject purge — order disk → Git → SQLite (the worktree path lives in `run_branch.worktree_path`, so the row goes last) and what happens if it crashes midway (running `delete` again finishes).
- **Logs** (`stream.jsonl`, `stderr.log` per Execution): do they stay raw next to the Execution and go away in cleanup? What minimal metadata stays in SQLite so `surt inspect` works after the stream is gone.

Out of v1 (do not decide here): redacting secrets in logs, opt-in full logs, "archive without deleting", cleaning up `.orphaned-*` directories, commits without a ref (`git gc` handles them), protection against junctions/`\\?\` beyond what `remove_dir_all` already gives.

## Context

- **States:** only `succeeded` and `cancelled` are terminal; a `blocked` run is active and cleanup does not touch it ([G. State Machines](../../../docs/architecture/G-state-machines.md)).
- **Revisions stay in SQLite** (blobs + Approvals) until an explicit purge; the Working Copy is only a local view and `surt show` reads Revisions after it is gone ([H. Artifact Storage](../../../docs/architecture/H-artifact-storage.md)).
- **Purge** = one `DELETE FROM subject` in a transaction (cascade) + GC of unreferenced snapshots/blobs; there is no `deleted_at` ([ADR 0009](../../../docs/adr/0009-history-is-purged-not-soft-deleted.md), [D](../../../docs/architecture/D-database.md#deleting-history)).
- **Git:** worktrees in `~/.surt/worktrees/<project-id>/<subject>/<key>/`, created with `--lock` (unlock before `remove`); removal goes through `git.lock`; cleanup never touches the Base or the user's worktree ([A](../../../docs/architecture/A-git-integration.md), [I](../../../docs/architecture/I-recovery.md)).
- **CLI:** `surt delete` confirms on a TTY showing the impact, `--yes` skips it, without a TTY and without `--yes` it refuses with exit `3`; reading never triggers cleanup; `surt logs` reads the stream from disk and, if it no longer exists, points to `inspect` ([J](../../../docs/architecture/J-cli.md)).
- **TOML:** the only per-workflow control is `[cleanup] on_success = "clean" | "keep"` (default `clean`).

Absorbed the former ticket 20 (Log policy, retention and redaction): for v1 it is enough to decide where the log lives and when it goes away.

## Answer

Resolved by human exchange in two rounds (9 questions, all accepted with the recommendation). Full section in [I. Cleanup, Delete and Logs](../../../docs/architecture/I-cleanup.md). Sections A (Git and process), D, E, F, H, I-recovery and J updated to point to it. `CONTEXT.md` gains **Cleanup** and **Purge**. No new ADR: no decision is hard to reverse (nothing changes the schema beyond one nullable column).

Final decisions:

- **When:** cleanup runs in the same command that writes the terminal state, still holding the run's lock (the Driver on `succeeded`; the Driver or `surt cancel` on `cancelled`). It is not a Step. New column `workflow_run.cleaned_at` (root only, terminal only). Terminal + clean policy + `cleaned_at` NULL = **cleanup pending**.
- **Failure:** each item is idempotent. A failure (e.g. a file in use on Windows) does not stop the others, does not change the exit code, writes `cleanup.incomplete` and warns with `retry:`. Who finishes it: any command that takes that run's lock (`feature`/`plan`/`start`/`cancel`/`delete`), never a read, never another Subject. Final warnings (do not leave it pending): unrecorded edit, Task Branch moved by someone else, `worktree_path` that is no longer a worktree of the repo.
- **`succeeded` + `clean`:** removes the Working Copy (except edits), `runtime/executions/<id>/`, the worktrees (`unlock` + `remove --force`; ignored files go with them), the Task Branches (with CAS on `tip_sha`) and the Salvage Refs. The Integration Branch, SQLite and the logs stay. With `keep` nothing goes, `status` shows `kept` and only `delete` cleans up.
- **`cancelled`:** removes what weighs on disk (Working Copy, Execution directories and worktrees, first saving leftovers to a Salvage Ref) and keeps every ref (Integration Branch, Task Branches, Salvage Refs) until `surt delete`. `on_success` does not apply.
- **Logs:** in `.surt/logs/<subject>/<execution-id>/{stream.jsonl,stderr.log,output.log}` (`output.log` = full output of the Checks). They survive every cleanup and go away only on `delete`. `runtime/executions/<id>/` keeps only `inputs/` and `delivery/`. For `inspect` without a stream, `runtime_info` gains `exit_code` and `num_turns` (JSON, no new column).
- **Unrecorded edit in the Working Copy:** cleanup never deletes it; it keeps the file and warns. `--new-run` refuses with exit `3` while it exists. `delete` removes it and lists it.
- **`surt delete`:** refuses with exit `3` if there is an active run (`fix: surt cancel`). Keeps the Integration Branch of a `succeeded` run (it may have an open PR) and deletes that of a `cancelled` run. Task Branches and Salvage Refs go without CAS (the confirmation listed them). Never touches the Base or the user's worktree.
- **`delete` order:** `try_lock` on every root run of the Subject → worktrees, refs (one `update-ref --stdin`) and directories → `BEGIN IMMEDIATE`, which rereads the runs and refuses if a concurrent `--new-run` appeared, then the purge → release and remove the lock files. A failure in the Git/disk step skips the purge and exits with `1` + `retry:`. Running it again finishes the job. No `deleting` marker.
- **Out of v1:** removing each Task's worktree as soon as it is integrated (there is a single cleanup, at the end of the root run).

Schema change (migration 0001 not yet released, edited): `workflow_run.cleaned_at` with `CHECK (cleaned_at IS NULL OR (parent_step_run_id IS NULL AND state <> 'active'))`. New event kinds: `cleanup.finished`, `cleanup.incomplete`. Validation: the migration runs on `sqlite3` 3.51 with `-bail`; the CHECK accepts a cleaned terminal run and refuses `cleaned_at` on an active run; the cascading purge removes everything. Fact verified on Git 2.56: `worktree remove` accepts a worktree with only ignored files and deletes the directory; untracked files require `--force`; a locked worktree requires `unlock`.

Context notes added to [Rust workspace structure and core traits](15-rust-workspace-and-traits.md), [Test strategy](16-test-strategy.md) and [Roadmap and Milestone 1](17-roadmap-and-milestone-1.md). No new ticket.
