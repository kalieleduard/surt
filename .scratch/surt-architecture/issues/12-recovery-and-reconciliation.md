# Recovery and reconciliation strategy

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Blocked by: 05, 09, 11, 18
Assignee: Kaliel

## Question

How does Surt resume any workflow — days later, after a crash, reboot or kill — without duplicating work or trusting old Claude sessions?

Decide: at which points there is a transactional checkpoint; the intention → effect → confirmation pattern for external operations (git, filesystem, process); detection of orphaned executions (PID + start time? expired lease?) and what to do with their partial results (dirty worktree, commit made but not recorded); reconciling worktrees and branches against SQLite; what `surt resume` does step by step; how attempt increments after a technical failure and the retry limit; Claude Code network/rate-limit errors; idempotency of each native operation.

Output: section I and the recovery part of section G.

## Context

Arose from [Git worktree/merge mechanics and partial states](04-git-worktree-merge-partial-states.md): git commands are not idempotent by themselves — reconciliation is "read state → compare with the record → act"; use the document's table of 21 partial states as the basis; the `update-ref` CAS rule (new → done, old → redo, other → stop); worktree `locked initializing`, an orphaned `index.lock` and merge/rebase markers as mandatory cases; the risk of `git gc` pruning missing worktrees after 3 months in runs resumed days later.

From [Process model and concurrency control](05-process-and-concurrency-model.md), already decided: orphan = an Execution in a running state whose root run lock is free; orphans are **never adopted** — the driving command kills the surviving group (identity pid + start time + boot id) and creates a new Attempt. There is no `surt resume`: rerunning `surt feature`/`plan`/`start` resumes. The first Ctrl-C lets the Native Operation in progress finish; only the second Ctrl-C (or kill -9/crash) leaves partial state to reconcile. Still missing here: what to do with the dirty worktree/unrecorded commit the orphan or the Interruption left behind.

From [Git integration model](09-git-integration-model.md) ([A. Git Integration](../../../docs/architecture/A-git-integration.md)): partial states to reconcile per Operation: `git.create_integration_branch` (does the branch exist?), `git.create_task_worktree` (branch without worktree, worktree `locked initializing`), `git.integrate` (ref = M → done; = T → redo; other → stop; the Task's worktree left at `detach M` in the middle of the re-check; commit M without a ref is harmless garbage), `git.merge_into_base` (`merge --ff-only` interrupted in the user's worktree). Integration Branch checked out in another worktree = technical failure, retried on the next command; tip diverging from the recorded one = **stop without adopting**, with the `git branch -f` instruction. Decide here the run's state in that case and how resuming confirms the tip was restored.

From [Builder ↔ Reviewer loop state machine](10-builder-reviewer-loop.md) ([G. State Machines](../../../docs/architecture/G-state-machines.md)): an `interrupted` Execution (Ctrl-C or an orphan stopped by the next Driver) **does not count** toward `max_attempts = 3`; `failed` counts. Failures that retrying cannot fix (auth, model, launch, Integration Branch moved externally/checked out in a worktree) take the Step Run to `blocked` on the 1st occurrence; decide here which network/rate-limit errors wait instead of becoming an Attempt, and complete the list of non-retryable ones. A driving command that takes the lock moves every `blocked` in the run back to `running` with a fresh budget. There is no `failed` run ([ADR 0008](../../../docs/adr/0008-limits-block-instead-of-failing.md)).

From [SQLite schema](11-sqlite-schema.md) ([D. Database Schema](../../../docs/architecture/D-database.md)): reconciliation compares Git with `run_branch.tip_sha` (CAS) and `worktree_path`; orphans are identified by `execution.pid` + `process_start_time` + `process_boot_id`. `git.integrate` now dates the merge commit with `step_run.started_at`, so every Attempt recomputes the same `M`. `synchronous = FULL` guarantees that a committed intention record survives a power cut; decide here what is written before each external effect (there is `event` for auditing, but the engine never reads it to decide; the state must be in columns). Failure classes: `execution.failure_class` ∈ `retryable | not_retryable`, with a free-form `error_code`; if network/rate limit need a third class ("wait"), it is an additive migration on the CHECK (table rebuild).

## Answer

Resolved by human exchange over three rounds (14 questions, all accepted with the recommendation). Complete section in [I. Recovery and Reconciliation](../../../docs/architecture/I-recovery.md); recovery part of [G. State Machines](../../../docs/architecture/G-state-machines.md) updated; trade-off in [ADR 0010 — Every Attempt starts from a clean worktree; leftovers are salvaged, never continued](../../../docs/adr/0010-every-attempt-starts-from-a-clean-worktree.md). New terms in `CONTEXT.md`: **Recovery**, **Intention**, **Salvage Ref**, **Transient Failure**.

Final decisions:

- **Recovery is not a mode.** No `surt doctor`/`repair`/`resume`: every driving command, when it takes the lock, does *preflight → lock → live Executions in the database → Intentions → Integration Branch check → unblock → scheduling*. Every Operation is "read → compare → act", and the next Attempt is the reconciliation.
- **Only what the Driver recorded as finished is accepted.** A valid delivery from an Execution whose end was not recorded is discarded (`interrupted`, without counting an Attempt), without revalidation.
- **Intention in a column before every external effect; recovery only reads and compares, never recomputes.** Creating a branch/worktree: the `run_branch` row is written first (`start_sha` = `tip_sha` = target). Moving a ref: new table **`ref_move`** (`step_run_id` PK, `ref_name`, `old_sha`, `new_sha`, `worktree_path`), which covers `git.integrate` and `git.merge_into_base` with one rule: ref = new → confirm; = old → redo; other → stop (integrate blocks with `integration_branch_moved`; merge_into_base yields `skipped`). It replaced the idea of `run_branch.pending_tip_sha`, because the Base is not a `run_branch`.
- **Working Copy:** `working_copy_file.pending_sha256`/`pending_revision_id` written at publication, promoted after the write (temp + rename); the scan distinguishes a completed write, a write not done and a human edit within the window, which becomes a human Revision after the agent's.
- **Worktree preparation before every Attempt** (agent or Operation, crash or not; ADR 0010): single target = the run's branch at `run_branch.tip_sha` (last accepted tip, recorded in the publication transaction); removes the worktree's locks, saves everything that diverges in a commit under `refs/surt/salvage/<execution-id>` (temporary index), clears markers (`--quit`), `checkout --force -B` + `clean -fd` (ignored files preserved). Human commits on the Task Branch also go to the salvage. The terminal warning only appears for unexpected leftovers.
- **Worktree states fixed automatically** (branch without worktree, `locked initializing`, deleted directory, moved repo → `worktree repair`, deleted metadata → rename to `.orphaned-<ts>` and recreate); it only blocks when the path belongs to another owner. Worktrees created with `--lock` against the 3-month `gc`.
- **Git locks:** Surt only deletes a lock provably its own with a dead owner (a run's worktree with no live Execution; a `<ref>.lock` of `surt/…` under `git.lock`); foreign ones become `failed` `retryable` with the file named.
- **Integration Branch moved or checked out:** the command refuses at startup without changing state (`status` stays `INTERRUPTED`); during the run, the Operation blocks (`not_retryable`), and the restoration is confirmed by ref = `tip_sha`, with no extra column.
- **`merge_into_base` interrupted in the user's worktree:** Surt never writes in it; the `ref_move` with `worktree_path` stays, and the Step Run blocks (`merge_into_base_interrupted`) with *finish* / *undo* instructions.
- **Transient Failure:** a third `failure_class = 'transient'` (`rate_limit`, `overloaded`, `server_error`, network), does not count an Attempt and pauses **all** of the Driver's agent launches until the reported reset or a 1→2→4→8→15 min backoff, with no cap (foreground, Ctrl-C). `authentication_failed`, `billing_error`, `model_not_found` and `invalid_request` are `not_retryable`; an unknown category is `retryable`. Operations are never `transient`.
- **Without a Driver:** `surt cancel` runs the orphan and Intention stages before cancelling (the history says where the branch is); `surt status` only reads and warns about a live orphan.
- Documented residual risk: a crash between the spawn and recording the pid leaves an unidentified process on macOS; it does not affect the result (acceptance is tied to recorded SHAs).

Schema changes (migration 0001 has not shipped yet, so it was edited): `ref_move` (18 tables), pending columns in `working_copy_file`, `'transient'` in the `execution.failure_class` CHECK, new `event` kinds. Validation: the migration runs in `sqlite3` 3.51 with `-bail`; the new CHECKs refuse `old_sha = new_sha`, a half-set pending pair and an unknown class.

Context notes added to [Cleanup and lifecycle safety](13-cleanup-delete-and-logs.md), [CLI surface and UX](14-cli-surface-and-ux.md), [Test strategy](16-test-strategy.md) and [Capture a real Claude Code stream as a fixture](19-real-claude-code-stream-fixture.md). No new ticket.
