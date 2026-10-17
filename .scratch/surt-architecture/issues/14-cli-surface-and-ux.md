# CLI surface and UX

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Blocked by: 05, 08
Assignee: Kaliel

## Question

What are the commands, their contracts and the follow-along experience?

Decide: the semantics of each command in the brief (especially `surt feature` × `surt plan` — what does each one do?); how a feature/run is referenced (`20261015-...`, UUID prefix?); `status`/`inspect` output (human × `--json`); live `logs` of several parallel executions (tail of `stream.jsonl`? multiplexed?); foreground progress; non-interactive mode (CI) and exit codes; actionable error messages; `surt init` (what it generates, `.gitignore`, built-in agents materialized or not); destructive confirmations.

Output: section J.

## Context

From [Agent contract](07-agent-contract.md): approving the manifest covers its index and every per-Task `.md`, and editing any document requires a new Revision and re-approval. The UX must allow inspecting that whole set; it must not present approval of an index alone as approval of the instructions. Agent contract and configuration: [F. Agent Contract](../../../docs/architecture/F-agent-contract.md).

From [Process model and concurrency control](05-process-and-concurrency-model.md), already decided: `surt feature` drives up to the spec gate; `surt approve` **only records**; `surt plan` drives the Task Agent up to the tasks gate (requires an approved spec); `surt start` drives up to the Shipper's delivery (requires approved tasks); rerunning the same command resumes (no `surt resume` in M1); `surt cancel` writes a request through SQLite; a command that finds the run with a live Driver refuses with the pid and instructions. Still missing here: feature/run reference, `status`/`inspect` output (owner, slots `4/4 in use`, `QUEUED`), exit codes, `logs`.

From [Workflow TOML schema](06-workflow-toml-schema.md): rejecting at an Approval Gate is its own command, **`surt reject <feature> -m "<reason>"`** (not `surt approve --reject`). The rejection triggers a new Iteration of the gate's Loop (the agent redoes the work reading the rejected Revision + the reason). Decide here: whether `reject`, like `approve`, only records, and which command drives the regeneration afterwards (rerun `surt feature <feature>` / `surt plan <feature>`?).

From [Artifact storage, freezing and revalidation](08-artifact-storage-freezing-revalidation.md): new commands/flags to specify: `surt show <subject> <artifact> [--rev N]`, `surt restore <subject> <artifact>`, `surt approve --rev N` (lock), `surt feature --name <slug>`; every feature action scans the Working Copy first; refusal messages sketched (spec not approved, invalid Item List edit, pinned Revision). `surt init` generates `.surt/.gitignore` (only `config.toml` and `agents/` versioned). See [H. Artifact Storage](../../../docs/architecture/H-artifact-storage.md).

From [Git integration model](09-git-integration-model.md) ([A. Git Integration](../../../docs/architecture/A-git-integration.md)): flags to specify: `surt feature --base <ref>` and a per-run override of `[git] merge_into_base` (e.g. `--merge`), applied when freezing the config. `status` shows the Base (`base main @ a11c9d0`), the Integration Branch and its tip, the effective merge mode, and per Task the merged SHA or the conflict reason (`merge: src/auth.rs` / `checks: test`). Refusal messages already sketched in the section: old Git, no identity, branch checked out in a worktree, tip moved externally, `not merged: main moved since base`. `surt show <subject> delivery` shows the Shipper's text.

From [Builder ↔ Reviewer loop state machine](10-builder-reviewer-loop.md) ([G. State Machines](../../../docs/architecture/G-state-machines.md)): `status` shows the run's derived state (`RUNNING`, `AWAITING APPROVAL`, `BLOCKED`, `INTERRUPTED`) and per Task the blocked step with the last error and the command to resume (mock in section G). The driving command exits with an error when it stops at `blocked` (define the exit codes here), and on resume prints `↻ unblocking … (3 more attempts)`. Consolidate here where `max_attempts` (3) and `max_iterations` (5) go in `.surt/config.toml`. There is no per-Task cancellation.

From [SQLite schema](11-sqlite-schema.md) ([D. Database Schema](../../../docs/architecture/D-database.md)): ids are UUIDv7 in TEXT (the same in the `runtime/executions/<id>/` directories and in the locks); Revisions appear as `rev N` (`artifact_revision.number`); the `actor` of Approvals, edits and cancel = `git user.email` (fallback: system user), useful for `approved by … at …`. The `surt inspect` timeline comes from `event` (initial kinds listed in section D). The reason for a block in `status` comes from `step_run.blocked_reason` + the last Execution's `error_message` (example query in section D).

From [Recovery and reconciliation strategy](12-recovery-and-reconciliation.md) ([I. Recovery and Reconciliation](../../../docs/architecture/I-recovery.md)): settle the text and exit code of: refusal at startup with the Integration Branch moved or checked out in another worktree (no state changes; `status` stays `INTERRUPTED`); recovery lines (`⚠ previous driver … is gone — recovering`, orphan stopped, Execution discarded, integration confirmed); salvage warning (only when the leftover is unexpected); pause on Transient Failure (`⏸ agent launches paused until 15:00`); the `merge_into_base_interrupted` block with its two instructions; `status` showing `⟳ integrating … (pending)` and a live orphan without a Driver; `cancel` without a Driver printing what recovery found.

## Answer

Resolved by human exchange in three rounds (18 questions, all accepted with the recommendation). Full section in [J. CLI Surface](../../../docs/architecture/J-cli.md). Sections A, D, G, H, I and E updated to point to it; `CONTEXT.md` (**Interruption**) now mentions the stop request from another terminal. No new ADR.

Final decisions:

- **M1 surface:** `init`, `feature`, `plan`, `start`, `approve`, `reject`, `stop`, `cancel`, `status`, `inspect`, `logs`, `show`, `restore`, `delete`. The brief's `resume` is rerunning the phase's command; the brief's `stop` becomes **`surt stop`**: a remote Interruption through SQLite (new column `stop_requested_at`/`_by` in `workflow_run`, kind `run.stop_requested`). The request applies only to the Driver alive when it is written and is cleared by whoever takes the lock. `surt delete` is only a reserved name.
- **Reference:** full name, slug without the date if unique, or a unique prefix; ambiguity refuses with a list; never an implicit "current feature". Writes act on the active run; reads use the active or the latest one, with `--run N`. Task by key; Execution by id/prefix ≥ 8. Surt always prints full names.
- **`surt feature`:** the argument is a reference only if it is exactly an existing full name. A description whose slug matches a feature with an active run refuses (even after midnight) instead of creating `-2`; `--new` forces it. `surt feature <name>` resumes the spec phase (regenerates after a reject; "already approved" exits 0). A new run of the same Subject only with `--new-run` (most recent request, Base = `HEAD` or `--base`, refreezes everything). `--merge`/`--no-merge` only on `surt feature`.
- **`approve`/`reject` only record** and end with the next command (`next:`/`regenerate:`). The gate → command table lives in the CLI layer of the `feature` workflow, not in the TOML schema nor in the engine.
- **Reading never writes:** `status`/`inspect`/`show`/`logs` do not write a Revision, do not recreate a file, do not take a lock — they show `✎ … edited`. Fixes the "When" list of [H. Edit detection](../../../docs/architecture/H-artifact-storage.md#edit-detection), which included the read commands.
- **Formats:** `status` as a list (active runs; `--all`) and as detail (Base, branch, merge mode, gate showing the whole approved set); `inspect` at three levels (run, task, Execution); `show` with an index, `--file`, `--task --iter`; `logs` rendered/multiplexed with `-f`/`--raw`; append-only progress + a live footer on a TTY, a liveness line every 5 min without a TTY.
- **Channels:** progress on stderr, data on stdout; `--json` (`"schema": 1`) only on `status`/`inspect`/`show`; without a TTY it never prompts; `NO_COLOR`; ASCII fallback for symbols; `SURT_LOG` for diagnostics.
- **Exit codes:** `0` ok · `1` internal · `2` usage · `3` refused with no change · `4` blocked · `5` cancelled meanwhile · `130` interrupted (Ctrl-C or `surt stop`). The same on all three OSes.
- **Confirmations:** `cancel` and `restore` (and `delete`) confirm showing the impact; `--yes`; without a TTY and without `--yes` = `3`.
- **Error messages:** `✖ what` + context ≤ 2 lines + state only when not obvious + 1–3 ways out with fixed labels (`fix:`, `next:`, `resume:`, `regenerate:`, `undo:`, `inspect:`, `or:`).
- **`surt init`:** explicit (the others refuse outside a project), at the root of the main worktree, with a Git preflight, idempotent, without materializing built-in agents. Refuses inside a Surt worktree.
- **Consolidated config:** `.surt/config.toml` with `max_concurrent_agents`, `max_attempts`, `max_iterations` at the top, `[git] merge_into_base`, `[[checks]]`; global `~/.surt/config.toml` (`%USERPROFILE%` on Windows) with `stop_grace_period`. An unknown key is an error. J is the single reference.

Schema change (migration 0001 not yet released, edited): `workflow_run.stop_requested_at`/`stop_requested_by` with a pair CHECK and a "root only" CHECK. Validation: the migration runs on `sqlite3` 3.51 with `-bail`; the request is written and cleared, and a half pair is refused by the CHECK.

Context notes added to [Cleanup and lifecycle safety](13-cleanup-delete-and-logs.md), [Rust workspace structure and core traits](15-rust-workspace-and-traits.md), [Test strategy](16-test-strategy.md), [Roadmap and Milestone 1](17-roadmap-and-milestone-1.md) and [Log, retention and redaction policy](13-cleanup-delete-and-logs.md) (later absorbed by 13). No new ticket; the "config contents" fog was consolidated and left the map.
