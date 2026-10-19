# Spec: Milestone 2 — operate and debug a run without opening SQLite

Labels: ready-for-agent
Type: spec
Status: open (13 of 16 tickets resolved; 14, 15 await the first Windows CI run, 16 awaits CI on three OSes and the manual Claude Code smoke)
Map: [Surt — Architecture and implementation plan](../surt-architecture/map.md)
Plan: [K. Roadmap](../../docs/architecture/K-roadmap.md#after-milestone-1), [J. CLI Surface](../../docs/architecture/J-cli.md), [I. Cleanup, Delete and Logs](../../docs/architecture/I-cleanup.md)
Previous: [Milestone 1](../milestone-1/spec.md)

## Problem Statement

Milestone 1 delivers a Feature end to end and survives a `kill -9`. Once something goes wrong, though, the developer is mostly on their own:

- **A blocked or rejected Task can't be explained from the CLI.** `surt status` prints one log path. To find out why a review was rejected three times, the developer `tail`s a raw `stream.jsonl` full of JSON events, or opens `.surt/surt.db` in `sqlite3` and joins `event`, `step_run` and `execution` by hand. Even then the timeline is thin: every `event.data` is `{}`, four event kinds the plan lists are never written, and `execution.runtime_info` only holds the Claude Code version (no session, model, exit code, turns, usage or cost).
- **Watching a live run is blind.** Agent output is visible only by tailing files, with one file per Execution and new directories appearing per Attempt. In a terminal, progress is a stream of event lines with no sense of what is running now or for how long. In CI, a long Builder looks like a hang.
- **Stopping from another terminal is impossible.** Only a Ctrl-C in the driving terminal interrupts a run. A Driver started in a tmux pane or a CI job can only be cancelled for good, which throws the run away.
- **Undoing an edit is a hack.** To undo an unrecorded Working Copy edit (for example an edit to a Pinned Revision that every driving command refuses), the developer has to `rm` the file and rerun.
- **Starting a Subject over and cleaning up are missing.** After a cancel, `surt feature "<description>"` creates a `-2` Feature, so the same request ends up under two names. Features, logs, Task Branches and Salvage Refs pile up forever with no supported way to remove them.
- **Scripts parse text.** No read command has `--json`, so a CI script that wants to know which Task is blocked has to parse the human table. There is no way to read an earlier run (`--run`), a Task's artifact from a given Iteration (`--iter`) or one file of the manifest (`--file`).
- **Old Windows consoles show garbage.** A console that is not UTF-8 prints mojibake instead of `✔ ✖ ⚠`.

K's ordering puts this milestone before M3 for a reason: a blocked run in Milestone 1 already says what to do, but debugging a review rejected three times with only `tail` hurts every time.

## Solution

Milestone 2 ("Operate and debug") ships the rest of [J](../../docs/architecture/J-cli.md): all 14 commands and every read flag. With it, a developer can understand, steer and clean up any run from the CLI:

```
$ surt inspect add-login B                       # why is B stuck?
14:02:11  step.running        build iter 2
14:02:11  execution.started   attempt 1 (agent-0) · 0192f3a8
14:09:40  execution.failed    attempt 1 · invalid_delivery — final JSON missing "outputs"

$ surt logs 0192f3a8                             # what did the agent actually do?
Read src/auth.rs
Edit src/auth.rs (+12 −3)
"Added the OAuth callback; the expiry test is still missing…"

T2 $ surt stop add-login                         # stop the run driving in T1, keep it resumable
⏳ stop requested; driver pid 48213 is stopping 3 executions…
✔ stopped. resumable: surt start 20261017-add-login

$ surt restore add-login spec                    # undo an edit to a pinned spec
$ surt feature 20261017-add-login --new-run      # start the same request over as run 2
$ surt delete 20261015-old-idea                  # purge a Feature, keep delivered branches
$ surt status --json | jq '.features[] | select(.state=="BLOCKED") | .name'
```

A live `surt start` in a terminal gets a footer that is redrawn in place. Without a TTY it prints a line after every 5 minutes without an event. A non-UTF-8 console gets ASCII symbols.

No migration: `0001_initial.sql` already has `stop_requested_at/by`, `event.data`, `execution.runtime_info` and the cascades the purge relies on. The engine stays generic. `inspect`, `logs`, `restore`, `stop`, `delete` and new runs are Subject/run/Execution concepts. The phase vocabulary (spec, manifest, Task) is still known only to the `surt` crate.

## User Stories

### Seeing what happened: `surt inspect`

1. As a developer, I want `surt inspect <feature>` to print the run's timeline (creation and Base, Revisions published, Approvals with their reasons, Driver starts, stops and recoveries, cleanup), so that I can see how the run got to where it is without opening SQLite.
2. As a developer, I want `surt inspect <feature> <task>` to print one Task's timeline (Iterations and why each started, Step Runs, every Attempt with its error code and message, blocks and unblocks), so that I can see why a Task is stuck or looping.
3. As a developer, I want `surt inspect <execution-id>` to show one Execution: its role, Task, step, Iteration and Attempt, its state and failure, its profile (runtime, model, effort, permission mode), session id, pid, exit code and duration, the Inputs it read (with Revision numbers and SHA-256), the worktree it ran in, its Salvage Ref and how to read its log, so that one screen tells me everything about one agent run.
4. As a developer, I want `inspect` to show Claude Code's reported model, turns, usage, cost and permission denials when the runtime reported them, and to leave out the values that weren't reported (never `0`), so that I can tell an expensive or stuck agent from a cheap one.
5. As a developer, I want `inspect` to work after cleanup and after the log files are gone, because it reads only SQLite, so that history stays explainable forever.
6. As a developer, I want an Execution referenced by its full id or by a unique prefix of at least 8 characters (as it appears in Salvage Refs, Execution directories and `surt` output), with an ambiguous or too-short prefix refusing and listing candidates, so that I can copy an id from anywhere.
7. As a developer, I want each timeline line to carry its time and, for older events, its date, so that I can line up a run that spans days.
8. As a developer, I want a run created by Milestone 1, whose events carry no details, to still render a timeline from kinds and times, so that upgrading never breaks `inspect`.

### Watching agents: `surt logs`

9. As a developer, I want `surt logs <feature>` to print the latest Execution of each Task, rendered as one line per tool call and the first line of each assistant message, so that I can skim what the agents did without reading JSON.
10. As a developer, I want `surt logs <feature> <task>` and `surt logs <execution-id>` to narrow it to one Task or one Execution, so that I can read exactly the run I'm debugging.
11. As a developer, I want `surt logs … -f` to follow every running agent of the run, multiplexed with a `B build#2 │` prefix, and to pick up new Executions (the next Attempt, the next Task) as they start, until the run's Driver stops, so that I can watch a live run from a second terminal.
12. As a developer, I want the source prefix to appear only when more than one source is printed, so that a single log reads cleanly.
13. As a developer, I want `--raw` to print the original `stream.jsonl` lines untouched, so that I can pipe them to `jq`.
14. As a developer, I want Operations that run processes (Checks, the integration re-check) to show their combined output with one header per Check, so that I can see why a Check failed.
15. As a developer, I want `logs` on an Execution whose log directory is missing to say so and point at `surt inspect <id>`, so that I'm never stuck on a deleted file.
16. As a developer, I want `logs -f` on a run with no live Driver to print what exists and exit `0` instead of waiting forever, so that it's safe in scripts.
17. As a developer, I want unknown stream event types and fields to be skipped by the renderer, not treated as errors, so that a newer Claude Code doesn't break `logs`.

### Reading any run, Revision and file

18. As a developer, I want `--run N` on `status`, `inspect`, `logs` and `show` to select run `N` of the Subject, defaulting to the active run or the latest one, so that I can look at a cancelled run after starting a new one.
19. As a developer, I want `--run N` for a run that doesn't exist to refuse and list the Subject's runs with their states, so that I know which numbers are valid.
20. As a developer, I want `surt show <feature> <artifact> --task <key> --iter N` to print the Revision produced in Iteration `N` of that Task, so that I can compare the feedback of review 1 with review 2.
21. As a developer, I want `--iter` without `--task` to be invalid usage (exit `2`), so that the flags can't be combined in a way that means nothing.
22. As a developer, I want `surt show <feature> manifest --file tasks/B.md` to print one file of a multi-file Revision byte for byte, so that I can read one Task's instructions as approved.
23. As a developer, I want an unknown `--file` path to refuse and list the files of that Revision, so that I can correct the path at once.

### Machine-readable output: `--json`

24. As a script author, I want `--json` on `status`, `inspect` and `show` to print one JSON document on stdout with `"schema": 1`, so that I can react to a run's state without parsing text.
25. As a script author, I want `status --json` to carry each Feature's run, derived state, Driver, gates (Revision, origin, SHA-256, Approval) and Tasks (state, Iteration, Attempt, merged SHA, blocked reason, last error, the command to resume), so that a CI job can tell exactly what is blocked and why.
26. As a script author, I want `inspect --json` to return the timeline events and the Execution detail as structured fields, so that I can compute durations or costs.
27. As a script author, I want `show --json` to return the Revision's metadata (number, origin, SHA-256, Approval) together with its files' content, so that one call gives me what was approved and what it said.
28. As a script author, I want refusals under `--json` to keep the human `✖` message on stderr and the exit code unchanged, with nothing on stdout, so that `set -e` and `jq` behave predictably.
29. As a script author, I want fields only added within `"schema": 1` and any removal or rename to bump it, so that my scripts don't break silently.
30. As a script author, I want timestamps in JSON as RFC 3339 UTC strings and ids as full strings, so that `jq` and other tools compare them without conversion.
31. As a developer, I want `--json` to be rejected on driving commands (exit `2`), so that nobody writes a script that expects a document from `surt start`.

### Stopping from another terminal: `surt stop`

32. As a developer, I want `surt stop <feature>` to make the live Driver perform the same Interruption as a first Ctrl-C (agents stop gracefully, a running Operation finishes) and exit `130`, so that I can pause a run driving in a tmux pane, a CI job or another terminal.
33. As a developer, I want `surt stop` to report the Driver's pid and how many Executions it is stopping, wait until it has stopped and print the resume command, so that I know when it's safe to close my laptop.
34. As a developer, I want the stopped Driver's terminal to say it was stopped by `surt stop` from another terminal and print the resume command, so that whoever is watching that terminal understands what happened.
35. As a developer, I want `surt stop` with no live Driver to refuse with exit `3` and show the run's derived state, so that I learn there is nothing to stop.
36. As a developer, I want `surt stop` to ask no confirmation, because the run stays resumable, so that it's quick.
37. As a developer, I want a stop request whose Driver died before reading it to be cleared by the next driving command when it takes the lock, so that a stale request never stops the next Driver.
38. As a developer, I want stopped Executions to never count toward `max_attempts`, the same as a Ctrl-C, so that stopping is free.
39. As a developer, I want a stop request to be recorded with who asked and when, and to appear in `inspect`, so that the timeline explains an interruption I didn't cause.

### Undoing an edit: `surt restore`

40. As a developer, I want `surt restore <feature> <artifact>` to rewrite the Working Copy from the latest recorded Revision and discard my unrecorded edit without creating a Revision, so that I can back out of a mistaken edit.
41. As a developer, I want `restore` to first show me what will be lost (`tasks/B.md +4 −1`, `tasks/C.md new file`, a deleted file) and ask `[y/N]`, so that I never discard work by accident.
42. As a developer, I want `--yes` to skip the question and, without a TTY and without `--yes`, `restore` to refuse with exit `3`, so that scripts never hang.
43. As a developer, I want `restore` to remove files that aren't part of the Revision and recreate missing ones, so that the Working Copy ends up exactly equal to the Revision.
44. As a developer, I want `restore` when nothing differs to say so and exit `0`, so that rerunning it is harmless.
45. As a developer, I want `restore` to refuse while a Driver is live on the run, naming its pid, so that it never races a Revision being published.
46. As a developer, I want `restore` on an artifact without a Working Copy (no Approval Gate) to refuse and list the artifacts that can be restored, so that I don't guess.
47. As a developer, I want the pinned-edit and revalidation refusals to suggest `undo: surt restore <feature> <artifact>` instead of `rm <file>`, so that the way out is one copyable command.
48. As a developer, I want `restore` on a terminal run to refuse and say the file is kept from an ended run (`fix: move or delete the file`), so that `restore` never rewrites the Working Copy of a run that is over.

### Starting over: `surt feature <feature> --new-run`

49. As a developer, I want `surt feature <feature> --new-run` to start run `N+1` of the same Subject from its latest `request` Revision, so that a cancelled or delivered Feature can be redone under the same name.
50. As a developer, I want the new run to take the current `HEAD` (or `--base`) as its Base and to freeze definitions, agents and configuration again, so that it reflects the project as it is now.
51. As a developer, I want the new run's branches to be `surt/<feature>.N` and `surt/task/<feature>.N/<key>` and its worktrees to be under `<feature>.N`, so that it never renames or touches the previous run's branches.
52. As a developer, I want `surt feature <feature>` on a Subject with no active run to refuse and suggest `--new-run`, showing how the last run ended, so that I never start over by accident.
53. As a developer, I want `--new-run` to refuse while the Subject has an active run, so that a Subject keeps at most one active run.
54. As a developer, I want `--new-run` to refuse while the Working Copy holds an unrecorded edit from the previous run, listing the files, so that the new run never overwrites something I typed.
55. As a developer, I want `--new-run` to finish a pending cleanup of the previous run before creating the new one, so that leftovers never mix with the new run's.
56. As a developer, I want `--new-run` to drive the new run to the spec Approval Gate like `surt feature "<description>"` does, so that starting over is one command.
57. As a developer, I want the `cancel` confirmation to tell me that starting over needs `surt feature <feature> --new-run`, so that I know the way back before I confirm.
58. As a developer, I want every read command's default to move to the new run as soon as it exists, while `--run 1` keeps the old one readable, so that history isn't lost.

### Removing a Feature: `surt delete`

59. As a developer, I want `surt delete <feature>` to purge the Subject (every run's worktrees, Task Branches, Salvage Refs, the Integration Branches of runs that didn't succeed, logs, Working Copy, Execution directories and every SQLite row), so that old Features stop cluttering my disk, ref list and `status --all`.
60. As a developer, I want the Integration Branch of every `succeeded` run to be kept, so that deleting history never deletes delivered work.
61. As a developer, I want `delete` to show the impact first (runs by state, refs deleted and kept, log size, unrecorded edits, Revision/Execution/event counts) and ask `[y/N]`, so that I know exactly what goes.
62. As a developer, I want `--yes` to skip the question and, without a TTY and without `--yes`, `delete` to refuse with exit `3`, so that scripts never hang.
63. As a developer, I want `delete` to refuse with exit `3` while the Subject has an active run and point at `surt cancel`, so that `delete` never stops agents.
64. As a developer, I want `delete` to refuse with exit `3` when any of the Subject's run locks is busy, so that it never races a Driver finishing a cleanup.
65. As a developer, I want a `delete` that fails partway through Git or disk work (a file in use on Windows) to leave the database untouched, exit `1` and print `retry: surt delete <feature>`, so that running it again finishes the job.
66. As a developer, I want `delete` to refuse if a new run of the Subject appeared while it was removing files, so that it never purges a run whose disk state it didn't clean.
67. As a developer, I want the purge to also drop snapshots and blobs that nothing references any more, so that the database shrinks with the history.
68. As a developer, I want `delete` to remove the run lock files after the purge, so that `.surt/runtime/locks/` doesn't grow forever.
69. As a developer, I want `delete` to never touch the Base, my own worktrees or any path Surt didn't record, so that a purge can't damage my repository.
70. As a developer, I want a kept run's status line to suggest `surt delete` as the way to remove it, so that I can find how to clean up.

### Live progress in the terminal

71. As a developer, I want a driving command on a TTY to show a footer under the event lines, redrawn in place, with what is running and for how long, what is queued, the Driver's slots and the total elapsed time, so that I can see at a glance that the run is alive.
72. As a developer, I want the footer erased before each event line and redrawn after it, and removed before the final summary, so that the scrollback holds only event lines.
73. As a developer, I want a driving command without a TTY to print `… still running: B build (15m), C review (3m)` after every 5 minutes without an event, so that CI doesn't assume a hang and kill the job.
74. As a developer, I want no footer, colours or spinner without a TTY, so that CI logs stay clean.
75. As a developer on an old Windows console or a non-UTF-8 locale, I want the symbols to become `[ok] [error] [warn] [retry] [paused] [blocked]` and every other non-ASCII glyph an ASCII equivalent, so that the output stays readable.
76. As a developer, I want `NO_COLOR` to keep disabling colour independently of the symbol set, so that the two settings don't interfere.

### Status after M2

77. As a developer, I want a blocked step in `surt status` to point at `surt inspect <execution>` and `surt logs <execution>` instead of a raw file path, so that the way to investigate is a command.
78. As a developer, I want `surt status <feature>` on a Subject with several runs to say which run it shows and how many exist, so that I know `--run` is there.
79. As a developer, I want the `status` and `show` output, its exit codes and the guarantee that reading never writes to hold for every new flag, so that `watch surt status --json` is safe.

### Surt maintainer

80. As a Surt maintainer, I want every state change's event to carry its display details in `event.data`, written in the same transaction, and never read by the engine to decide anything, so that timelines are complete without coupling behaviour to the audit log.
81. As a Surt maintainer, I want the event kinds D lists for features in v1 up to M2 (`run.stop_requested`, `driver.stopped`, `driver.recovered`, `working_copy.edit_refused`, `intention.confirmed`, `intention.discarded`) to be emitted, so that the timeline has no hidden gaps.
82. As a Surt maintainer, I want the Claude Code adapter to record the full `runtime_info` (version, session id, reported model, requested effort, exit code, turns, usage, cost, permission denials) and the Fake to record what it simulates, so that `inspect` has something to show and tests can check it.
83. As a Surt maintainer, I want the footer, the 5-minute line, the ASCII mapping and the `logs` renderer to be pure functions of their inputs and `now`, so that they're tested without a TTY and without waiting 5 minutes.
84. As a Surt maintainer, I want CI to stay green on macOS, Linux and Windows with the line-coverage gate above 90%, so that M2 keeps M1's quality bar.
85. As a Surt maintainer, I want M2 to need no migration, so that a Milestone 1 database opens unchanged.

## Implementation Decisions

The plan documents ([J](../../docs/architecture/J-cli.md), [I. Cleanup](../../docs/architecture/I-cleanup.md), [I. Recovery](../../docs/architecture/I-recovery.md), [H](../../docs/architecture/H-artifact-storage.md), [D](../../docs/architecture/D-database.md), [A. Process Model](../../docs/architecture/A-process-model.md)) already decide most of M2, and this spec consolidates them. Points the plan left open are marked **decided here**. The implementation writes them back into the plan document named, so that the rule "the plan wins on divergence" still holds.

### Scope boundary with M3/M4

- M2 ships every command and flag of J **except** `--merge`/`--no-merge` (M4). The `agent slots` line of `surt status` stays out until M3 (cross-process slots). The footer shows the Driver's own slot count.
- The Transient Failure still blocks at once (M3). `restore`, `inspect` and `logs` work the same once the pause arrives.

### Engine facade (`Project`)

- New facade methods, matching [B](../../docs/architecture/B-workspace.md#the-project-facade): `request_stop`, `restore`, `delete`, `inspect`. Like M1's `cancel_impact`, they come with read companions the CLI needs for confirmations: a restore impact (per file: changed with line counts, new, missing) and a delete impact (runs by state, refs to delete and keep, directories with sizes, unrecorded edits, row counts). They are added to the "B facade method list" rule exception the same way as M1's extra reads.
- `create_run` gains the "new run of an existing Subject" form. It takes the Subject's latest `request` Revision as a new human-origin Initial Artifact, a Base (`HEAD` or `--base`) and a fresh freeze. It refuses while the Subject has an active run (already a schema invariant). It also refuses while the previous run's Working Copy differs from its baseline. Run ordinals and the `.N` branch stem already exist in the Git code.
- **`--new-run` and pending cleanup:** the CLI calls `drive` on the previous root run first, which finishes the pending cleanup under that run's lock and returns at once because the run is terminal. Then it calls `create_run`. This adds no new primitive.
- **Read targets:** read methods accept a run selector (active-or-latest, or ordinal `N`) and an Execution reference (full id or unique prefix of at least 8 characters). Resolution lives in the engine next to Subject resolution. Ambiguous or unknown references are `Refusal`s with candidates.
- All new methods keep M1's error model: a `Refusal` carries observed values and no text, and the CLI owns the wording and the exit code.

### Stop

- `request_stop` writes `stop_requested_at/by` and a `run.stop_requested` event in one transaction, but only when a Driver is live. Liveness is decided the same way `status` decides it today. With no live Driver it refuses with the run's derived state.
- The Driver reads the request on its next tick (the same reload that already picks up cancel requests) and turns it into the graceful `StopSignal`, so a stop and a first Ctrl-C share one code path. It ends with `DriveOutcome::Interrupted` (exit `130`), with a progress event that says the stop came from another process.
- Clearing: every driving command clears `stop_requested_at/by` in the transaction where it records itself as the Driver after taking the lock ([J](../../docs/architecture/J-cli.md#stop-and-cancel)).
- `surt stop` then waits for the Driver to release the lock, the same way `surt cancel` with a live Driver waits. A Ctrl-C on `surt stop` leaves the request in place. **Decided here:** the wait has no timeout, because the Driver's own grace period bounds it.

### Restore

- **Decided here:** `restore` acts only on the Subject's **active** run and takes that root run's lock with `try_lock`. A live Driver refuses with its pid. Before rewriting, it settles a pending Working Copy write for that artifact with the same code reconciliation uses, so it never mistakes a half-published agent Revision for an edit.
- The result is exactly the latest Revision's file set. Files outside the Revision are removed, missing files are recreated and changed files are rewritten. The baseline then equals the file hashes. No Revision, Approval or pin changes. One event records it.
- Only artifacts with a Working Copy (those behind an Approval Gate) can be restored. Others refuse and list the ones that can.
- On a terminal run, `restore` refuses with I's existing `fix: move or delete the file` wording ([I. Cleanup](../../docs/architecture/I-cleanup.md#unrecorded-working-copy-edits)).
- The H revalidation refusals change their way out from `fix: rm <file>, then rerun` to `undo: surt restore <feature> <artifact>`, as H always intended.
- **Line counts for the confirmation:** a small line-based diff count computed in-house (added and removed lines per file). No new dependency is added to B's table.

### Delete

- The order follows [I. Cleanup](../../docs/architecture/I-cleanup.md#surt-delete):
  1. `try_lock` every root run lock of the Subject (any busy lock refuses).
  2. Git and disk, each step idempotent:
     - worktrees: `unlock` and `remove --force`, pruning a missing entry;
     - one `update-ref --stdin` transaction deleting Task Branches, Salvage Refs and non-delivered Integration Branches at whatever SHA they hold;
     - directories: logs, Working Copy and Execution directories.
     A failure there finishes the rest of the step, skips the purge and exits `1` with `retry:`.
  3. One `BEGIN IMMEDIATE` transaction rereads the set of root runs, rolls back and refuses if it changed, then purges: `DELETE FROM subject` and the garbage collection of unreferenced snapshots and blobs ([D](../../docs/architecture/D-database.md#deleting-history)).
  4. Release the locks and remove the lock files, best effort.
- "Delivered" means the root run is `succeeded`. Its Integration Branch is kept. A `cancelled` run's Integration Branch is deleted.
- `delete` refuses on an active run (`fix: surt cancel <feature>`) and never stops agents.
- There is no `deleting` marker. An interrupted delete is finished by running it again, and until then `status` shows the Feature as it is.

### Inspect, events and `runtime_info`

- `inspect` reads only SQLite (`event`, Execution rows, Inputs, `run_branch`), plus a read-only `git for-each-ref` for the Execution's Salvage Ref. It never needs a log file.
- **Decided here:** `event.data` carries display-only details for each kind: Base and actor on `run.created`; artifact, Revision number and origin on `revision.published`; decision, Revision, actor and the first line of the reason on `approval.recorded`; pid on `driver.started`/`driver.stopped`; Attempt, slot and runtime on `execution.started`; code and message on `execution.failed`; reason on `step.blocked`; the new budget base on `step.unblocked`; Iteration and cause on `iteration.started`; what was found on `driver.recovered`; and so on. The exact field list per kind is written into D's events section as part of this milestone. The engine never reads `event.data` (D's rule stands), and a missing field renders as the bare kind.
- The missing kinds are emitted where their state change already happens: `run.stop_requested`, `driver.stopped` (on every Driver exit that releases the lock normally), `driver.recovered` (when reconciliation found something), `intention.confirmed`/`intention.discarded` (reconciliation of `ref_move` and `run_branch` Intentions) and `working_copy.edit_refused`. `agents.paused`/`agents.resumed` wait for M3.
- The Claude Code adapter writes the full `runtime_info` at the end of an Execution: version, session id, reported model, requested effort, exit code, turns, usage, cost and permission denials, with unknown values absent. The Fake writes the subset it simulates.

### Logs

- `logs` reads `.surt/logs/<subject>/<execution-id>/` ([I. Cleanup](../../docs/architecture/I-cleanup.md#logs)) and is otherwise read-only. Execution selection (latest per Task, a Task, an Execution) comes from the engine's read methods.
- **Rendering is a pure function in the `surt` crate** from one `stream.jsonl` line to zero or one display line: a tool call becomes `<Tool> <main argument>`, an assistant message becomes its first line in quotes, and an accepted delivery becomes `✔ delivered …`. Tool results and unknown types render nothing. `output.log` and `stderr.log` lines pass through.
- `-f` polls the files (bounded intervals, no file-watcher dependency) and the engine's Execution list for new Executions. It stops once the run's Driver is gone and the files are drained. With no live Driver at start, it prints what exists and exits `0`.
- **Decided here:** the engine exposes each Execution's log directory path. The CLI opens the files, because rendering is a CLI concern and the engine stays text-free.

### `--json`

- Exists on `status`, `inspect` and `show` only. Driving commands reject it at the clap level (exit `2`).
- One document per call on stdout with `"schema": 1`. The shapes are serialisations of the same read views the human output renders. They are not a second query path.
- **Decided here:** timestamps are RFC 3339 UTC strings, ids are full strings, and derived states use the same uppercase names as the human table (`RUNNING`, `AWAITING APPROVAL`, `BLOCKED`, `INTERRUPTED`, `SUCCEEDED`, `CANCELLED`). In `show --json`, a file's content is a UTF-8 string, or `content_base64` when the bytes aren't valid UTF-8. A J subsection with one example document per command records the field list.
- Refusals and errors keep the human stderr message and exit code, and print nothing on stdout.

### Terminal output

- The engine adds a typed **activity snapshot** to `Progress`, emitted when it changes: running steps with their start times, the queued count, and slots in use and limit. **Decided here:** the engine already holds this on every tick, so the CLI doesn't rebuild it from individual events.
- The footer, the 5-minute line and the symbol set are pure functions of (last snapshot, time of the last event line, `now`, terminal capabilities) inside the existing progress module of the `surt` crate. A Tokio interval in the CLI only calls them.
- Footer: only when stderr is a TTY. It is drawn with `\r` plus ANSI erase-line (B: no `indicatif`/`crossterm`), erased before each event line and before the final summary.
- 5-minute line: only without a TTY, after every 5 minutes without an event line.
- **ASCII symbols, decided here:** the question "does the console output UTF-8" is answered in the engine's `platform` module (Windows: console output code page ≠ 65001; Unix: the first set of `LC_ALL`, `LC_CTYPE`, `LANG` doesn't name UTF-8) and re-exported through the facade. This keeps every OS call inside `platform`. J fixes the six mapped symbols (`[ok] [error] [warn] [retry] [paused] [blocked]`), and the implementation maps every other glyph the CLI prints (`●`, `⏳`, `⏹`, `✎`, `⟳`, `│`, `─`, `…`, `−`) to ASCII.
- `NO_COLOR` and the symbol set are independent.

### CLI changes to M1 commands

- `feature <feature>` with no active run refuses and suggests `--new-run` ([J](../../docs/architecture/J-cli.md#surt-feature-create-resume-or-start-again), rule 4).
- The `cancel` confirmation gains the `--new-run` line.
- Blocked steps in `status` print `inspect:` and `logs:` commands instead of a log path.
- `status <feature>` names the run it shows when the Subject has more than one.
- A kept run says `remove with: surt delete <feature>`. This only becomes reachable with M4's `keep`, but it costs one match arm.
- `show` gains `--iter` (requires `--task`), `--file`, `--run` and `--json`.

### Build order

Each slice is a PR on `master` with CI green on the three OSes:

| Slice | Contents | Done when |
|---|---|---|
| **R1 History** | `event.data` per kind, the missing event kinds, full `runtime_info`, the run selector and Execution prefix resolution, `inspect` at three levels, `--run` on `status`/`show` | `surt inspect` explains the brief's E2E (rejected review, new Builder) through the binary |
| **R2 Logs** | stream renderer, `logs` with `-f`/`--raw`, multiplexing, missing-log refusal; `status` points at `inspect`/`logs` | `logs -f` follows two Tasks of a live Fake run through the binary |
| **R3 Full show and `--json`** | `--iter`, `--file`, JSON documents for `status`/`inspect`/`show`, the J subsection | `status --json` reports a blocked Task, and reads still write nothing |
| **R4 Stop** | `request_stop`, Driver handling, clearing on lock, `surt stop` | a stop from a second process ends the Driver with `130`, and the run resumes |
| **R5 Restore** | restore impact, `restore`, the H refusal wording | a pinned-edit refusal is undone with `surt restore` and the run continues |
| **R6 New run** | `create_run` for an existing Subject, the `--new-run` refusals, cancel confirmation text | cancel → `--new-run` → spec gate with `surt/<feature>.2` |
| **R7 Delete** | delete impact, the four-step order, purge with garbage collection, lock files | delete after a cancelled and a succeeded run keeps only the delivered branch, and an interrupted delete finishes on rerun |
| **R8 Terminal** | activity snapshot, footer, 5-minute line, UTF-8 detection and ASCII mapping | the pure render tests pass, and the manual TTY check on macOS passes |

R1 comes first because `logs` and `status` point at `inspect`. R2–R8 are independent of each other once R1 lands, except that R6's confirmation text and R7's impact reuse R1's run selector.

## Testing Decisions

### What makes a good test here

- A test asserts **external behaviour** at a seam. It checks exit codes, the contract lines on stderr, stdout documents, and final state in Git (refs kept or deleted, worktrees) and SQLite (rows present or purged, `stop_requested_at`, `event.data` fields). It doesn't assert call order, private names or intermediate structs.
- Store, Git and the clock are real, as in M1: file-backed SQLite in a tempdir, a temporary repository per test. Time-dependent rendering takes `now` as an argument. There are no fixed sleeps, and process waits are bounded polling, as in M1's recorded `testing.md` exception.
- Each behaviour covers success, refusal and edge cases (`rules/rust.md`, `rules/testing.md`).

### Seams (confirmed)

1. **The `surt` binary: the main seam.** Uses the M1 harness (sandbox repository, the Fake via `SURT_RUNTIME`, direct reads of Git and SQLite). It covers:
   - every new command and flag, and its exit codes (`0`, `2` for `--iter` without `--task` and `--json` on a driving command, `3` for each refusal, `1` for a delete that fails partway, `130` for a stopped Driver);
   - confirmations without a TTY (`restore`/`delete` refuse without `--yes`);
   - `stop` from a second process against a Driver whose Fake Execution waits until stopped;
   - `logs -f` following a live run;
   - JSON documents parsed with `serde_json`, with their fields checked;
   - and, extending M1's test, **reads never write**: the database and Working Copy hashes are unchanged across `status`, `inspect`, `logs` and `show` with every flag.
2. **The `Project` facade.** Engine tests with the typed Fake script, real store and Git. They cover:
   - stop-request semantics: a request with no Driver refuses, a leftover request is cleared on lock, and stopped Executions spend no Attempt;
   - `restore`: it creates no Revision, settles a pending Working Copy write, refuses with a live Driver, and refuses on a terminal run;
   - `create_run` for a new run: `.N` branches and worktree paths, the refusal on an unrecorded edit, and the refusal on an active run;
   - the `delete` order: a busy lock refuses; a removal failure leaves rows intact and the rerun finishes; a run set that changed between step 2 and the purge rolls back; garbage collection leaves no unreferenced snapshot or blob; delivered Integration Branches survive;
   - `inspect` views built from `event.data` and `runtime_info`, including a run whose events have `{}` (M1 data).
3. **Pure rendering in the `surt` crate**, extending the existing progress message tests. It covers:
   - the footer text for a given activity snapshot and `now`;
   - when the 5-minute line fires;
   - that the ASCII mode output contains only ASCII bytes for every message;
   - the `logs` renderer over the **real Claude Code fixtures** already in the engine's test fixtures (success, auth failure, nonexistent model, SIGTERM), plus synthetic lines for unknown types.
4. **`ClaudeCodeRuntime` against fixtures** (existing seam). It checks that the full `runtime_info` is extracted from the captured streams, with unreported values absent.

### Modules tested

| Module | Seam |
|---|---|
| `project` (new facade methods, run/Execution resolution) | `Project` and binary |
| `store` (stop request, purge and garbage collection, event data) | `Project` |
| `drive` (stop handling, activity snapshot, new event kinds) | `Project` |
| `artifacts` (restore, restore impact) | `Project` |
| `cleanup` (delete) | `Project` |
| `reconcile` (`driver.recovered`, `intention.*` events) | `Project` (M1's fault-point tests gain assertions on events) |
| `runtime::claude_code` (`runtime_info`) | fixtures |
| `platform` (UTF-8 console detection) | unit test on each OS in CI |
| `surt` crate (commands, JSON, footer, heartbeat, ASCII, logs renderer) | binary and pure render |

### Prior art

- The binary tests in the `surt` crate's CLI test suite, especially the `status`, `show`, `cancel`, `interrupt` and `kill` files: sandbox setup, Fake scripts, a second process against a live Driver, and DB/Working Copy hashing for "reads never write".
- The facade tests under the engine's project tests (`drive/stopping`, `drive/cancelling`, `drive/cleanup`, `drive/working_copy`, `drive/pinning`). Stop, restore and delete follow their shape.
- The progress message tests in the `surt` crate's driving module, the template for the pure render tests.
- The Claude Code fixture tests in the engine's runtime tests, the template for the `runtime_info` and `logs` renderer tests.

### Done when

1. `cargo fmt --all -- --check`, `cargo clippy --all-targets --features test-support -- -D warnings` and `cargo test --features test-support` pass in CI on macOS, Linux and Windows, with the line-coverage gate above 90%.
2. A manual check on macOS, appended to `docs/smoke-claude-code.md`. With the real Claude Code it covers: the footer redraws during `surt start`; `surt logs -f` renders a live Builder; `surt stop` from a second terminal leaves a resumable run; `surt inspect <execution>` shows session, model, turns and cost.

## Out of Scope

- **M3:** the Transient Failure pause with reset time and backoff (Milestone 1 behaviour stays: it blocks at once); agent slots across processes and the `agent slots` line of `status`; the global `~/.surt/config.toml` (`stop_grace_period` stays fixed at 10 s); `agents.paused`/`agents.resumed` events.
- **M4:** `.surt/agents/` overrides, `merge_into_base = "ff-only"`, the `merge` step and `--merge`/`--no-merge`, `on_success = "keep"`.
- **Not in v1 at all ([I. Cleanup](../../docs/architecture/I-cleanup.md#not-in-v1)):** log redaction, opt-in full logs, rotation and retention; archiving without deleting; removing `.orphaned-*` directories; removing a Task's worktree as soon as it is integrated.
- **Outside the map:** a daemon and `--detach` (`logs -f` reads files, it doesn't attach to a process); `--json` on driving commands; a TUI; external integrations; a second real runtime; cancelling or deleting a single Task.
- **Manual validation on Linux and Windows** beyond CI. The ASCII fallback is unit-tested, not checked on a real legacy console.

## Further Notes

- **No migration.** Everything M2 writes already has a column: `stop_requested_at/by`, `event.data`, `execution.runtime_info`, and the cascades plus `NO ACTION` cross-references the purge relies on. A Milestone 1 database opens unchanged. Its events simply lack details.
- **Plan updates that are part of this milestone.** M1 left K's "replacements for cut commands" as the current behaviour. With M2 they go away. The implementation updates:
  - K: mark M2 shipped and remove the replacement notes;
  - J: the `--json` example documents, `restore`'s active-run-only rule, and stop's wait without timeout;
  - D: the `event.data` field list per kind;
  - H: the refusal wording now names `surt restore`;
  - I. Cleanup: `restore` on a terminal run;
  - B's facade rule exception: the new read companions.
- **Engine genericity.** Nothing in M2 names Spec, Task, Builder, Reviewer or Shipper in `surt-engine`. Commands that print "task B" do so in the `surt` crate from item keys, as M1 does.
- **Next after M2:** M3 Run unattended, then M4 Fit the project. v1 is the end of M4.
