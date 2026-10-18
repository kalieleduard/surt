# Spec: Milestone 1 — an end-to-end Feature that survives `kill -9`

Labels: ready-for-agent
Type: spec
Status: open (19 of 22 tickets resolved; 03, 20 await the first Linux/Windows CI run, 22 awaits the manual Claude Code smoke)
Map: [Surt — Architecture and implementation plan](../surt-architecture/map.md)
Plan: [K. Roadmap and Milestone 1](../../docs/architecture/K-roadmap.md)

## Problem Statement

The developer wants to deliver a small Feature with AI agents without babysitting a chat session: write the request, review the spec, review the Task plan and get back a ready branch, with every Task built, reviewed and integrated. Nothing today does this in a local, deterministic and resumable way. The repository only has a throwaway `ferris-says` crate. Manual Claude Code sessions lose the thread when the terminal closes. Nobody guarantees that an unreviewed commit didn't get into the branch. A Ctrl-C, a `kill -9` or a reboot in the middle of the work leaves worktrees, branches and processes in a state the human has to untangle alone.

The A–L plan in `docs/architecture/` resolved every v1 decision. What is missing is the code. This spec describes **Milestone 1**: the smallest subset of v1 that delivers a Feature end to end with the real Claude Code and survives a `kill -9` at any point.

## Solution

A local-first `surt` binary in Rust, with SQLite as the source of truth, that drives the built-in `feature` workflow (with one `task-cycle` per Task) in three foreground phases, each stopping where it needs a human:

```
$ surt init
$ surt feature "add login"             # Spec Agent → stops at the spec gate
$ $EDITOR .surt/features/20261017-add-login/spec.md     # optional
$ surt approve add-login
$ surt plan add-login                  # Task Agent → manifest + tasks/*.md → gate
$ surt approve add-login
$ surt start add-login                 # builders, checks, review, integrate, shipper, cleanup
✔ task A merged (3c1aa20)
↻ task B build attempt 2
✔ task B merged (9e04b7d)
✔ shipper    delivered — surt/20261017-add-login @ 4e1b2c0
✔ run        succeeded
↻ cleanup    2 worktrees · 2 task branches · working copy
```

The result is an Integration Branch `surt/<feature>` into which each Task entered as a merge commit of the SHA that passed the Checks and was approved by the Reviewer, plus the Shipper's delivery text. Nothing is pushed and the main branch is not touched. Any interruption, from Ctrl-C to a power cut, is resumed by running the phase command again: Recovery happens inside it, with no special mode.

The engine (`surt-engine`) is generic and knows nothing about Spec, Task, Builder, Reviewer or Shipper. The `surt` crate holds the CLI and the built-in workflow.

## User Stories

### Project and installation

1. As a developer, I want `surt init` to create `.surt/` at the toplevel of my repository's main worktree, so that every later command finds the project from any subdirectory.
2. As a developer, I want `surt init` to check Git ≥ 2.38 and a configured user identity, and refuse with a fix if either is missing, so that I learn about prerequisites before the first agent spends tokens.
3. As a developer, I want `surt init` to be idempotent and never overwrite `config.toml` or `agents/`, so that rerunning it is safe.
4. As a developer, I want `surt init` to write a `.surt/.gitignore` that versions only `config.toml` and `agents/`, so that the database, logs, Working Copies and runtime files never pollute my Git history.
5. As a developer, I want `surt init` to generate a commented `config.toml` with the defaults (`max_concurrent_agents = 3`, `max_attempts = 3`, `max_iterations = 5`, `merge_into_base = "never"`, an example `[[checks]]`), so that I can see what I can tune.
6. As a developer, I want every other command to refuse with exit `3` outside an initialized project and point me at `surt init`, so that no command silently creates state somewhere unexpected.
7. As a developer, I want a command run inside a Surt-managed worktree under `~/.surt/worktrees/` to refuse, so that I don't accidentally drive a Feature from a Builder's scratch space.
8. As a developer, I want unknown keys in `.surt/config.toml` to be errors, so that a typo never silently falls back to a default.
9. As a developer, I want Surt to work the same on macOS, Linux and Windows, including locks, process trees, paths and exit codes, so that my team can use it on any machine.

### Creating and specifying a Feature

10. As a developer, I want `surt feature "<description>"` to create a Feature named `YYYYMMDD-<slug>` from my description, so that Features sort by date and are easy to refer to.
11. As a developer, I want `--name <slug>` to override the derived slug, so that I can pick a short name for a long description.
12. As a developer, I want a Feature name that collides with an existing one to get an automatic `-N` suffix, so that I never have to invent a unique name by hand.
13. As a developer, I want Feature names that are reserved on Windows (`con`, `nul`, …) or not portable to be rejected or adjusted, so that a Feature created on macOS also works on Windows.
14. As a developer, I want `--base <ref>` to fix the commit my Feature starts from, with `HEAD` as the default, so that the Integration Branch is created exactly where I chose.
15. As a developer, I want the run to freeze the workflow definitions, the agents and the project configuration when it is created, so that editing `config.toml` later never changes a run already in progress.
16. As a developer, I want `surt feature "<description>"` to drive the Spec Agent and stop at the spec Approval Gate with exit `0`, so that I review the spec before anything else happens.
17. As a developer, I want rerunning `surt feature "add login"` while that slug has an active run to refuse and tell me how to resume or create anew, so that pressing ↑ Enter after a Ctrl-C never creates a duplicate Feature.
18. As a developer, I want `--new` to skip that guard, so that I can deliberately start a second Feature with the same description.
19. As a developer, I want `surt feature <full-name>` to resume the spec phase of the active run, regenerating after a rejection, so that resuming is "run the phase command again".
20. As a developer, I want `surt feature <full-name>` on an already approved spec to say so, point at `surt plan` and exit `0` without changing anything, so that rerunning it is harmless.
21. As a developer, I want `surt feature "<description>"` after a cancel to create a `-2` Feature, so that I can start over without `--new-run`, which Milestone 1 doesn't ship.

### Reviewing, editing, approving and rejecting

22. As a developer, I want the spec to appear as `.surt/features/<feature>/spec.md`, so that I can read and edit it in my editor.
23. As a developer, I want my edit to the spec to become a new human-origin Revision when I run `approve`, `reject` or a driving command, so that my change is versioned and the Approval targets exactly what I saw.
24. As a developer, I want `surt approve <feature>` to record an Approval on the waiting gate's latest Revision, print what it approved with its SHA-256 and author, and print the next command, so that I always know how to continue.
25. As a developer, I want `surt approve`/`reject` to never drive anything, so that recording a decision is instant and never launches an agent by surprise.
26. As a developer, I want `--rev N` to refuse if the latest Revision is not `N`, so that I never approve a Revision I didn't read.
27. As a developer, I want `surt reject <feature> -m "<reason>"` to record the rejection with my reason and point at the command that regenerates, so that the agent redoes the work reading my reason.
28. As a developer, I want a rejected spec to start a new Iteration of the spec loop whose Spec Agent reads the rejected spec and my reason, so that the next Revision addresses my feedback.
29. As a developer, I want loops that contain an Approval Gate to have no Iteration limit, so that I can reject as many times as I need.
30. As a developer, I want a deleted Working Copy file to be recreated from the latest Revision by the next command that acts, so that deleting a file is how I undo an unrecorded edit while `surt restore` isn't available.
31. As a developer, I want an edit to an approved but not yet consumed Revision to be recorded as an unapproved Revision, with every driving command refusing until I approve it, so that unapproved content never flows downstream.
32. As a developer, I want an edit to a Pinned Revision to be refused rather than recorded, with the Driver letting running work finish, launching nothing new and exiting `3`, so that implementation never runs against content different from what Builders already read.
33. As a developer, I want those refusals to say `fix: rm <file>, then rerun`, so that I have a way out without `surt restore`.
34. As a developer, I want an agent Revision that arrives while I have an unrecorded edit to first save my edit as its own Revision and warn me, so that nothing I wrote is lost.

### Planning Tasks

35. As a developer, I want `surt plan <feature>` to refuse unless the spec is approved, so that the Task Agent never plans from an unapproved spec.
36. As a developer, I want `surt plan` to drive the Task Agent to the manifest Approval Gate, so that I review the Task breakdown before any code is written.
37. As a developer, I want the manifest and one Markdown file per Task to appear in `.surt/features/<feature>/manifest.json` and `tasks/<key>.md`, so that I can review and edit the Builder's instructions directly.
38. As a developer, I want approving the manifest to cover the JSON and every Task file together as one Revision, so that editing any `tasks/*.md` needs reapproval.
39. As a developer, I want an edit to the manifest set to be validated with the same rules as an agent delivery (schema, unique keys, valid `needs`, no cycles, every referenced file present, no unreferenced file, safe paths), and an invalid edit to create no Revision and refuse, so that a broken manifest never reaches implementation.
40. As a developer, I want rejecting the manifest to regenerate it with my reason, so that I can push back on Task size or ordering.

### Implementing, reviewing and integrating

41. As a developer, I want `surt start <feature>` to refuse unless the manifest is approved, so that no Builder runs before human approval.
42. As a developer, I want `surt start` to create the Integration Branch `surt/<feature>` at the Base, so that all reviewed work collects in one Surt-owned branch.
43. As a developer, I want each Task to run in its own worktree on its own Task Branch `surt/task/<feature>/<key>`, starting from the Integration Branch tip when the Task begins, so that Tasks never step on each other or on my working tree.
44. As a developer, I want Tasks to start in the order their `needs` allow, running independent ones in parallel up to `max_concurrent_agents`, so that a Feature finishes as fast as its dependencies permit.
45. As a developer, I want every Builder to be a fresh Claude session with only its fixed Inputs (its item, its `task.md`, the spec and any `feedback`/`check-report`/`conflict`), so that no stale context leaks between Attempts or Iterations.
46. As a developer, I want a Builder's delivery to be accepted only if its commit exists, is the worktree's `HEAD` and the Task Branch tip, descends from the Task's start (and from the conflict's integration tip after a conflict), is new, and leaves a clean worktree, so that a SHA-shaped string alone never counts as work.
47. As a developer, I want a Builder that finds nothing to change to make an empty commit explaining why, so that every integrated Task still has a reviewed SHA.
48. As a developer, I want my frozen `[[checks]]` to run as argv, without a shell, in the Task worktree on the Builder's commit, all of them even after one fails, so that the Builder sees every failure at once.
49. As a developer, I want a failed Check to skip review and start a new Iteration whose Builder reads the `check-report`, so that no Reviewer time is spent on a commit that doesn't pass.
50. As a developer, I want the Reviewer to judge the whole commit and its verdict to be bound to that exact SHA, so that an approval can never be reused for different code.
51. As a developer, I want an `approved` review with a blocking finding to be rejected as an invalid delivery, so that the verdict is always consistent.
52. As a developer, I want `changes_requested` to start a new Iteration whose Builder reads the feedback, and the next Reviewer to verify its earlier blocking findings without moving the bar, so that review converges.
53. As a developer, I want an approved Task to be integrated as a merge commit of the reviewed SHA, computed without a worktree and with deterministic dates, so that every Task's reviewed commit is provably in the Integration Branch's history.
54. As a developer, I want integration to rerun the Checks on the merge result when the Integration Branch moved since the Task started, so that every tip of the Integration Branch passed the Checks.
55. As a developer, I want a merge conflict or a failed re-check to send the Task back to its Builder with a `conflict` artifact, so that the Builder merges the tip, resolves and goes through check and review again.
56. As a developer, I want integration to move the Integration Branch only from the tip Surt recorded, so that commits made outside Surt never enter silently.
57. As a developer, I want the Shipper to run only after every Task merged, read the manifest and the integration snapshot, and deliver a `title` and `summary` without touching Git, so that I get a ready-made pull request description for a complete Feature.
58. As a developer, I want `surt start` to end with exit `0`, the delivered branch and SHA, and the run `succeeded`, so that scripts and humans both know it's done.

### Limits and `blocked`

59. As a developer, I want a step whose Executions fail technically to retry at once, up to `max_attempts` failed Attempts, and then become `blocked`, so that a flaky agent gets another chance but a broken one doesn't burn tokens forever.
60. As a developer, I want a Task Cycle that runs `max_iterations` Iterations without merging to become `blocked`, so that an endless review loop stops for a human.
61. As a developer, I want failures that retrying cannot fix (auth, missing or mismatched model, missing executable, Integration Branch moved or checked out, foreign worktree path) to block on the first occurrence, so that no Attempts are wasted.
62. As a developer, I want a rate limit, overload or network failure to block the step at once without spending an Attempt, with exit `4` and the reset time when the stream reports one, so that I know when to come back.
63. As a developer, I want a blocked Task to hold back only the Tasks that depend on it while independent Tasks keep running and merging, so that one stuck Task doesn't stop all work.
64. As a developer, I want a run to never "fail", only to be blocked until I resume or cancel it, so that no work is ever thrown away by a limit.
65. As a developer, I want `surt start` to exit `4` when it stops on a blocked step, listing each blocked step with its last error, its log path and the command to resume, so that I can investigate without opening SQLite.
66. As a developer, I want rerunning the phase command to unblock every blocked step with a fresh budget while keeping the counters' history (`attempt 4`, `iter 6`), so that resuming is my explicit decision and history stays honest.

### Interruption, cancellation and recovery

67. As a developer, I want Ctrl-C (or SIGHUP, or closing the console) to stop running agents gracefully, let a running Operation finish, and exit `130`, so that I can stop at any time and resume later.
68. As a developer, I want a second Ctrl-C to force-kill the agents' whole process trees, so that I'm never stuck waiting.
69. As a developer, I want interrupted Executions to never count toward `max_attempts`, so that pressing Ctrl-C five times never exhausts a step.
70. As a developer, I want a second driving command on a run that already has a live Driver to refuse with the Driver's pid, so that two processes never advance the same Feature.
71. As a developer, I want a `kill -9`, crash or reboot to release the run lock automatically, so that the next command can take over without manual cleanup.
72. As a developer, I want the next driving command to stop any agent left running by a dead Driver and mark it `interrupted`, so that no orphan keeps editing a worktree unattended.
73. As a developer, I want an Execution whose end was never recorded to be discarded even if it left a valid delivery, so that Surt never adopts work it didn't see finish.
74. As a developer, I want an integration that was interrupted between recording its Intention and confirming it to be confirmed if the ref moved, redone if it didn't, and blocked if someone else moved it, so that a crash never double-merges or loses a merge.
75. As a developer, I want every Attempt in a worktree to start from a clean worktree at the last accepted tip, with leftovers saved to a Salvage Ref and a warning naming it, so that half-done work is never continued but never lost either.
76. As a developer, I want a missing, half-created or orphaned Task worktree to be repaired automatically, and a path Surt didn't create to block instead of being deleted, so that Surt fixes what it owns and never touches what it doesn't.
77. As a developer, I want a driving command to refuse to start while the Integration Branch was moved or is checked out, with the exact `git` command to restore it, so that unreviewed commits never enter and I know how to fix it.
78. As a developer, I want a crash between publishing an agent Revision and writing its Working Copy file to never be mistaken for my edit, so that recovery doesn't invent human Revisions.
79. As a developer, I want `surt cancel <feature>` to confirm first, showing what will be stopped and what stays, so that I don't end a run by accident.
80. As a developer, I want `surt cancel` without a TTY and without `--yes` to refuse with exit `3`, so that scripts never hang on a prompt.
81. As a developer, I want `surt cancel` with no live Driver to stop orphans, settle pending Intentions so history says where the branch really is, and cancel the run, so that cancelling a crashed run is one command.
82. As a developer, I want `surt cancel` while another terminal drives the run to make that Driver stop and exit `5`, so that cancellation works across terminals through SQLite alone.

### Cleanup

83. As a developer, I want a `succeeded` run to remove the Working Copy, execution directories, Task worktrees, Task Branches still at their recorded tip and Salvage Refs, while keeping the Integration Branch, the history and the logs, so that my disk and ref list stay clean without losing anything accepted.
84. As a developer, I want a `cancelled` run to salvage and remove the worktrees but keep Task Branches and Salvage Refs, so that unintegrated work survives as cheap refs.
85. As a developer, I want a Working Copy file with an unrecorded edit, a Task Branch someone moved, or a foreign path to be kept with a warning instead of removed, so that cleanup never destroys my work.
86. As a developer, I want a cleanup that fails partway (a file held open on Windows) to stay pending and be finished by the next command that takes the run's lock, so that I never clean up by hand.

### Reading

87. As a developer, I want `surt status` to list every Feature with an active run, its derived state (`RUNNING`, `AWAITING APPROVAL`, `BLOCKED`, `INTERRUPTED`), its Driver and its progress, and `--all` to add terminal runs, so that I see everything at a glance.
88. As a developer, I want `surt status <feature>` to show the Base, the Integration Branch, each gate's Revision and Approval, and each Task's state, Iteration, Attempt and merged SHA, so that I understand where the run is.
89. As a developer, I want `surt status` at an Approval Gate to list every file the Approval covers and any unrecorded edit, so that I know what I'm approving.
90. As a developer, I want `surt status` to warn when an agent is still running unattended with no Driver, and to show pending integrations and cleanup pending, so that I never mistake an orphan for a stopped run.
91. As a developer, I want `surt show <feature>` to list the artifacts and `surt show <feature> <artifact> [--rev N] [--task <key>]` to print the content byte for byte to stdout, so that I can read any Revision after cleanup and redirect it to a file.
92. As a developer, I want `show` to warn on stderr when the Working Copy holds an unrecorded edit, so that I know the printed content isn't what's on disk.
93. As a developer, I want `status` and `show` to never take a lock, recover, record an edit, recreate a file or write anything, so that they're safe in `watch`, scripts and read-only checkouts.
94. As a developer, I want a `<feature>` argument to accept the full name, a unique slug or a unique prefix, and an ambiguous one to refuse listing the candidates, so that I type little without ever acting on the wrong Feature.

### Output and codes

95. As a developer, I want progress, warnings and errors on stderr and only data on stdout, so that `surt show … > file` writes only the content.
96. As a developer, I want every error to follow one shape (`✖` line, at most two context lines, a state line only when work happened, one to three labelled copyable commands with full names), so that I always know what failed and what to type next.
97. As a developer, I want exit codes `0` done, `1` internal error, `2` invalid usage, `3` refused (nothing changed), `4` blocked, `5` cancelled while driving, `130` interrupted, identical on the three OSes, so that scripts can react without parsing text.
98. As a developer, I want an unexpected error to print `✖ internal error: …` and suggest `SURT_LOG=debug`, so that I can report or debug it.
99. As a developer, I want `NO_COLOR` to disable colour, so that logs and CI output stay clean.
100. As a developer, I want `SURT_LOG` to enable Surt's own diagnostic logs on stderr for any command, so that I can debug Surt itself without a special build.
101. As a developer, I want each agent's stream and stderr written to `.surt/logs/<feature>/<execution-id>/`, surviving cleanup, so that I can read what an agent did with ordinary tools while `surt logs` isn't available.

### Visible Milestone 1 cuts

102. As a developer, I want a cut command or flag (`stop`, `inspect`, `logs`, `restore`, `delete`, `--new-run`, `--json`, `--run`, `--iter`, `--file`, `--merge`/`--no-merge`) to not exist at all and exit `2`, so that nothing half-works.
103. As a developer, I want `merge_into_base = "ff-only"` to refuse with exit `3`, so that I learn it isn't supported yet instead of thinking my branch was merged.
104. As a developer, I want driving commands to refuse with exit `3` when `.surt/agents/` exists, so that my overrides are never silently ignored.

### The real Claude Code

105. As a developer, I want every agent to run on my existing Claude Code subscription login, without an API key, so that using Surt doesn't change how I'm billed.
106. As a developer, I want each Execution to be a fresh session (`--session-id` new, `--no-session-persistence`, never `--resume`/`--continue`), so that no Attempt inherits a previous conversation.
107. As a developer, I want the agent's environment built from an allowlist rather than inherited, so that variables from my shell or a parent Claude session (gateway URLs, model overrides, API keys) never change auth or model.
108. As a developer, I want agents to run with `dontAsk`, explicit tools per role, MCP denied, hooks, slash commands, memory and CLAUDE.md loading disabled, so that each role can do exactly its job and nothing prompts me mid-run.
109. As a developer, I want an Execution accepted only when the process exited 0, initialization matches the requested session, model, version, permission mode and tools, the result is a success with `is_error = false` and a structured output, the envelope is valid and every declared file passes validation, so that a success-shaped stream is never mistaken for a delivery.
110. As a developer, I want Claude Code older than 2.1.301 or missing to be a launch error that blocks at once with an actionable message, so that I know to upgrade.
111. As a developer, I want stopping an agent to stop its whole process tree (process group on Unix, Job Object on Windows) with a 10-second grace period before a forced kill, so that no subprocess outlives Surt.

### Surt maintainer

112. As a Surt maintainer, I want CI to run `cargo fmt --check`, `cargo clippy --all-targets --features test-support -D warnings` and `cargo test --features test-support` on macOS, Linux and Windows with Git ≥ 2.38, so that every PR keeps the three platforms green.
113. As a Surt maintainer, I want a Fake Runtime that goes through the same delivery validation as Claude Code and can simulate valid deliveries, invalid JSON, missing files, model mismatch, crashes, limit failures and an Execution that waits until stopped, so that the whole flow is testable without spending tokens.
114. As a Surt maintainer, I want the Fake and fault points gated behind a `test-support` cargo feature that release builds never enable, so that test hooks can't reach users.
115. As a Surt maintainer, I want the brief's E2E to run through the binary in under 60 s per OS without real sleeps, so that the suite stays fast enough to run on every PR.
116. As a Surt maintainer, I want fault points at each of the eight transactional checkpoints and one real `kill` smoke, so that I can prove recovery from a crash between any Intention and its effect.
117. As a Surt maintainer, I want the engine crate to be unable to import the feature workflow, so that the compiler enforces that the core stays generic.
118. As a Surt maintainer, I want a written manual smoke checklist for the real Claude Code on macOS, so that "Milestone 1 done" is a repeatable check, not a feeling.

## Implementation Decisions

Every decision below is already resolved in the plan documents. This spec only consolidates them for M1. Any divergence between this text and `docs/architecture/` is settled by the plan document.

### Workspace and crates

- The `ferris-says` crate is replaced by a two-crate workspace ([ADR 0011](../../docs/adr/0011-two-crates-keep-the-engine-generic.md)):
  - **`surt-engine`** (lib) is the generic engine. It cannot depend on `surt`, and engine tests build synthetic workflows.
  - **`surt`** (bin) holds clap, the `plan`/`start` phase preconditions, the gate → next command table, all message text, the exit codes and the embedded assets (`feature`, `task-cycle` and the five Agents with TOML, instructions and schemas).
- Toolchain: `edition = "2024"`, `rust-version = "1.94"`, stable pinned in `rust-toolchain.toml`, multi-thread Tokio. The allowed dependencies are those in the table in [B](../../docs/architecture/B-workspace.md#toolchain-and-dependencies), which is a ceiling, not a floor. Forbidden: `async-trait`, `git2`/`gix`, `chrono`/`time`, `indicatif`/`crossterm`, `include_dir`, `dirs`.
- The `surt-engine` module split is fixed (`project`, `workflow`, `drive`, `reconcile`, `artifacts`, `context`, `runtime`, `operations`, `git`, `cleanup`, `store`, `platform`). File names inside each module may change. Every OS `#[cfg]` lives in `platform`.

### Engine interface: the `Project` facade

- The CLI sees a single public type, `Project`, and the types it returns. The rest of the engine is `pub(crate)`.
- Methods in M1: `open`, `find_subject`, `create_run`, `drive`, `approve`, `reject`, `cancel`, `status`, `show`. `request_stop`, `restore`, `delete` and `inspect` are left for M2 and need not exist.
- **`drive` is the whole sequence:** preflight → `try_lock` on the root run → reconcile (steps 3–6 of [I](../../docs/architecture/I-recovery.md#what-a-driving-command-does-when-it-takes-the-lock)) → advance until nothing can run without a human → cleanup if terminal. It is the only primitive that drives.
- Phases do not exist in the engine: `surt plan` checks "spec approved" in a `StatusView` before calling `drive`. Neither do confirmations: `cancel` acts when called, and the CLI asks beforehand.
- Subject resolution (full name → unique slug → unique prefix) lives in the engine.
- **Results and errors:** `DriveOutcome` = `AtGate` (0) · `Succeeded` (0) · `Blocked` (4) · `Cancelled` (5) · `Interrupted` (130) · `Halted(Refusal)` (3, after in-flight work finished). `Error` = `Refused(Refusal)` (3, nothing changed) · `Internal` (1). clap keeps 2. `Refusal` carries observed values and no text. The `✖ … / fix: …` wording belongs to the CLI. The engine uses `thiserror` and the CLI uses `anyhow` only on the `internal error` path.
- **Progress:** the engine never prints. It emits typed `Progress` events on a channel, and the CLI renders lines (no footer in M1). Internal diagnostics go through `tracing`, visible with `SURT_LOG`.
- **Stopping:** `StopSignal` with levels `None`/`Graceful`/`Force`. The first Ctrl-C/SIGHUP/console close triggers graceful and the second triggers force. A cancel request written to SQLite is read by `drive` on the next tick.

### Store

- All SQL lives in a single module. The schema is the plan's `0001_initial.sql`, unchanged and without comments: 17 tables, STRICT, UUIDv7 TEXT ids, INTEGER timestamps in Unix seconds written by Rust.
- Opening: WAL + `synchronous=FULL`, migrations under `db.lock`, and a database newer than the binary is refused (`SchemaTooNew`).
- There is **one method per transactional checkpoint** of [I](../../docs/architecture/I-recovery.md#transactional-checkpoints), not one repository per table. Each method is a `BEGIN IMMEDIATE` transaction with retry on `SQLITE_BUSY` that writes its rows and its `event` together. No transaction handle leaves the module.
- `load_run_graph` loads everything the scheduler reads, and `drive` reloads the `RunGraph` on every tick (~1 s), so Approvals and cancel requests from another process show up without caching.

### Pure scheduler

- Inside `drive`, deciding is separate from doing: `decide(&RunGraph, &Ctx) -> Vec<Action>`, with actions `StartIteration`, `ExpandItems`, `Launch`, `RunOperation`, `Block`, `FinishRun`, `Return(DriveOutcome)` and `Wait`.
- `RunGraph`, `Action`, `Ctx` and the state enums do not import `sqlx` or the `git` module. `store` depends on them, never the reverse (DIP exception recorded in [ADR 0012](../../docs/adr/0012-agent-runtime-is-the-only-trait.md)).

### The only trait: `AgentRuntime`

- It uses native `async`, is not object-safe, and `drive` takes `&impl AgentRuntime`.
- `ClaudeCodeRuntime` and `FakeRuntime` are public in `surt-engine::runtime`. The runtime launches, follows and stops an Execution. It does not resolve inputs, publish Revisions or write to SQLite.
- `RuntimeError` carries the `FailureClass` (`transient`, `not_retryable`, `retryable`) classified by the adapter. `drive` records that value on the Execution, and it never crosses the facade as an error.
- Store, Git, Operations and the clock have **no** trait: they are real in tests, and timers use `tokio::time::pause()`.

### Built-in Workflows and Agents

- The parser accepts the five `kind`s (`agent`, `approval`, `operation`, `loop`, `per_item`) with `deny_unknown_fields`, without expressions. Conditions only compare Outcomes, and `needs` is explicit.
- Static validation runs against the agents and the Operations registry. The root freezes the closure of definitions, agents and config with SHA-256 identity.
- The `Catalog` is an explicit table with `include_str!`, assembled by the `surt` crate. The engine embeds only its migrations and the schemas of the Operation artifacts (`base`, `check-report`, `conflict`, `integration`). The agent schemas (`manifest`, `commit`, `feedback`, `delivery`) travel with their Agents.
- `feature.toml` in M1 **has no `merge` step** (`git.merge_into_base` is not implemented). `[cleanup] on_success` only accepts `"clean"`.
- The five Agents share the same built-in profile: `claude-code`, `claude-sonnet-5-5`, effort `high`, auth `subscription`, `dontAsk`, with per-role tools as in [F](../../docs/architecture/F-agent-contract.md#permissions-and-tools). None of them limits turns or cost.
- The Subject name is `YYYYMMDD-<slug>` with an automatic `-N` suffix. The date comes from the local clock via `jiff`, and the name follows the portable-name rules.

### Artifacts and context

- Revision bytes are SHA-256 blobs in SQLite, with atomic publication. An Approval stores id + hash.
- A Working Copy exists only for artifacts with an Approval Gate, under `.surt/features/<feature>/`. Edit detection uses the full SHA-256 against the baseline, never mtime.
- The Working Copy's pending state (`pending_sha256`/`pending_revision_id`) is written in the publication transaction and promoted after the file is written.
- Pinning and revalidation follow [H](../../docs/architecture/H-artifact-storage.md#revalidation): editing an approved, not yet consumed Revision produces an unapproved Revision and makes the drivers refuse; editing a Pinned Revision is refused.
- Context Assembler: per-Execution inputs in `.surt/runtime/executions/<id>/` and the item file as a read-only `task.md`. Each Input carries a label with the Iteration and step that produced it, and from the second Iteration on the context says why the Iteration started.
- Delivery validation is the same for the Fake, Claude Code and human Item List edits: envelope `{outcome, outputs}`, relative paths with no escape or link, formats and schemas (draft-07), and consistency checks registered by the built-in workflow (Git evidence of the Builder's commit, `reviewed_commit` = `commit.sha`, `approved` with no blocking finding).

### Git and Operations

- The `git` CLI runs via argv with `LC_ALL=C`, minimum version 2.38, and `git.lock` with backoff for third-party locks.
- Five Operations in a closed `enum`: `git.create_run_branch`, `git.create_child_worktree`, `checks.run`, `git.integrate` and `git.snapshot_run_branch`. Every Operation follows "read state → compare with SQLite → act", so the next Attempt is its own reconciliation.
- `git.create_child_worktree` and `git.integrate` are serialized per root run (in-process mutex).
- Integration ([ADR 0007](../../docs/adr/0007-integrate-reviewed-sha-as-merge-commit.md)): `merge-tree --write-tree` → `commit-tree -p T -p R` dated by `step_run.started_at` → rerun the Checks if `T ≠ S` → `ref_move` Intention → `update-ref` with the expected old value → confirmation.
- Worktrees live in `~/.surt/worktrees/<project-id>/<feature>/<key>`, created with `worktree add --lock`. On Windows, Builders get `core.longpaths=true` via `GIT_CONFIG_*`.
- Every Attempt in a worktree first goes through preparation ([ADR 0010](../../docs/adr/0010-every-attempt-starts-from-a-clean-worktree.md)): locate → remove Surt's own locks → save leftovers to `refs/surt/salvage/<execution-id>` with a temporary index → clear in-progress operations → `checkout --force -B` + `clean -fd` → verify. The normal flow and recovery use the same code.

### Processes and concurrency

- One Driver per root run, guaranteed by a file lock at `.surt/runtime/locks/<root-run-id>.lock` via `File::lock` ([ADR 0002](../../docs/adr/0002-file-locks-over-sqlite-leases.md)), with no lease or heartbeat.
- Process identity is pid + native start time + boot id. Orphans are stopped, never adopted.
- Process trees use `process-wrap` (process group / Job Object with kill-on-close), with `PR_SET_PDEATHSIG` on Linux. `stop_grace_period` is fixed at 10 s.
- `max_concurrent_agents` is a **per-Driver** semaphore, read before each launch. `execution.slot` is still written.

### Recovery, cancellation and cleanup

- Recovery runs inside every driving command, never as a separate mode. It is the seven steps of [I](../../docs/architecture/I-recovery.md#what-a-driving-command-does-when-it-takes-the-lock), all in M1, including Salvage Refs and the refusal when the Integration Branch was moved or is checked out.
- In M1 a Transient Failure **blocks at once**: Execution `failed` with `failure_class = 'transient'` (no Attempt spent), Step Run `blocked` with `blocked_reason = 'not_retryable'`, exit 4, and the reset time appears in the message when the stream reports it. There is no pause or backoff.
- `cancel` with no Driver takes the lock, runs steps 3–4 and records the Cancellation. With a live Driver, it writes the cancel request and the Driver exits with 5.
- Cleanup runs after the terminal state, under the lock, and is idempotent. If it fails, it stays pending (`cleaned_at` NULL) and is resumed by the Feature's next `feature`/`plan`/`start`/`cancel`. The "what each ending removes" table in [I. Cleanup](../../docs/architecture/I-cleanup.md#what-each-ending-removes) applies to `succeeded`/`clean` and `cancelled`.

### Claude Code Runtime

- The launcher uses the argv from [F](../../docs/architecture/F-agent-contract.md#claude-code-launcher-and-compatibility) (`-p`, a new `--session-id`, `--no-session-persistence`, `--output-format stream-json --verbose`, full model and effort, `--permission-mode`, explicit tools and rules, a generated `--json-schema`, `--append-system-prompt-file`, a generated `--settings`, strict empty MCP, `--disable-slash-commands`, and `--safe-mode --setting-sources user` for subscription). The context goes through stdin, which is then closed.
- The child's environment is built from an **allowlist** plus the variables that disable auto-update, memory and CLAUDE.md. Write path rules use the `Edit(...)` namespace with the `//` prefix for an absolute POSIX root.
- The parser reads the stream until the process exits and the stream drains, ignores unknown types and fields, and does not assume `system/init` comes first. To accept, it requires the five criteria in [F](../../docs/architecture/F-agent-contract.md#runtime-success-and-failure); `subtype = "success"` alone proves nothing.
- Failure classification reads `assistant.error` and `terminal_reason`. An unknown category is `retryable`.
- Stream and stderr go to `.surt/logs/<feature>/<execution-id>/`.
- The minimum version is 2.1.301, and the actual version is recorded on each Execution.

### M1 CLI

- Ten commands: `init`; `feature "<description>"` with `--name`, `--base`, `--new`; `feature <feature>`; `plan`; `start`; `approve [--rev N]`; `reject -m [--rev N]`; `cancel [--yes]`; `status [<feature>] [--all]`; `show <feature> [<artifact>] [--rev N] [--task <key>]`.
- Cut commands and flags do not exist in clap.
- The next-command table (`approve-spec` → `plan`/`feature`, `approve-tasks` → `start`/`plan`) lives in the `surt` crate.
- `status <feature>` prints the log path and the resume command for each blocked step.
- Without a TTY nothing is asked. `cancel` without `--yes` refuses with 3.

### Build order

Each slice is a PR on `main` with green CI on the three OSes and a demonstrable "done" ([K](../../docs/architecture/K-roadmap.md#build-order)):

| Slice | Done when |
|---|---|
| S1 Skeleton: workspace, toolchain, CI, `platform`, `store`, `Project::open`, `init` | `surt init` works and is idempotent on the three OSes |
| S2 Workflow: parser, validation, freeze, `Catalog`, `create_run`, Subject name | `surt feature "add login"` creates the Subject and a `pending` run and stops without driving |
| S3 Engine + Fake: `decide`, `drive`, `artifacts`, `context`, trait + Fake + `test-support`, `approve`/`reject` | spec → gate → reject → spec rev 2 → approve → `plan` → gate, with the Fake, through the binary |
| S4 Git: `git`, five Operations, worktree preparation/salvage, `start` | the brief's E2E green (without `kill`) + the three Git scenarios |
| S5 Recovery and ending: `reconcile`, `ref_move`, Working Copy pending state, fault points, `kill` smoke, cleanup with retry, `cancel`, detailed `status`, `show` | the whole test-strategy suite passes |
| S6 Claude Code (in parallel from S3 on): launcher, allowlist, parser, classification, process trees, fixtures | the parser passes against the fixtures and the manual macOS smoke passes |

## Testing Decisions

### What makes a good test here

- A test asserts **external behaviour** seen from a seam: exit code, stderr lines that are part of the contract, final state in Git (refs, merge commits, worktrees) and in SQLite (run/step/execution states, Revisions, Approvals, `cleaned_at`). It does not assert the order of internal calls, private function names or the shape of intermediate structs.
- Store, Git and the clock are **real**: file-backed SQLite in a tempdir (never in-memory), a temporary Git repository per test, timers with `tokio::time::pause()` and pure functions that take `now`. The rule is the one in `rules/architecture.md`: test the domain without I/O and the adapters at the edges.
- No real `sleep`, and every test is deterministic on the three OSes. The Fake's Execution "that waits until stopped" replaces timing.
- Engine tests use **synthetic workflows**. `feature.toml` is exercised only through the binary.
- Each test covers success, error and edge cases, per `rules/rust.md`.

### Seams (confirmed)

1. **The `surt` binary, the main seam.** The brief's E2E runs here: `init` → `feature` → spec → reject → spec rev 2 → approve → `plan` → approve → `start` with 2+ Tasks, one with a rejected review and a new Builder → integration → Shipper → cleanup → verification of the final state by reading Git and SQLite directly. It also covers each exit code (0, 2, 3, 4, 5, 130) and the guarantee that `status`/`show` write nothing (hash of the database and the Working Copy before and after). The Fake is selected by `SURT_RUNTIME` with the path to a TOML script, only with the `test-support` feature. 60 s ceiling per OS.
2. **The `Project` facade.** Engine integration tests with a typed Rust script, indexed by the step's logical key + Iteration + Attempt (`Script::new().on("build", 1, 1, Behavior::Commit)`). It covers:
   - store fault points at the **eight checkpoints** (run created, Iteration/expansion, Execution started, process launched, Intention, Execution accepted, Working Copy written, Execution ended any other way): for each one, fail right after the commit and check that the next `drive` finishes the run without duplicating an effect;
   - the **real `kill` smoke**: kill the process during a waiting Fake Execution; resuming finishes the run, with the killed Execution `interrupted` and a new Attempt;
   - the **three Git scenarios**: clean merge, conflict (back to the Builder with `conflict`) and failing Checks (skips review, the Builder reads `check-report`);
   - schema invariants: ≤1 active root per Subject and ≤1 live Execution per Step Run, enforced by CHECK and a unique index;
   - a second Driver on the same run refuses, and `kill -9` releases the lock;
   - pinning and revalidation, edit detection and an invalid Item List edit;
   - full cleanup, pending cleanup after a removal failure, and its resumption by the next command.
3. **`decide(&RunGraph, &Ctx)`, table tests.** It covers G's hard rules: 3 Attempts → `blocked`; 5 Iterations without a gate → `blocked`; a Loop with an Approval Gate has no limit; `interrupted` and `transient` spend no Attempt; `not_retryable` blocks on the first one; the budget is renewed on unblock; dependents of a blocked Task wait and independent ones go on; a run only ends `succeeded`/`cancelled`; `Return` when nothing runs without a human.
4. **`ClaudeCodeRuntime` against real fixtures.** The captured streams (success, auth failure, nonexistent model, SIGTERM 143 without `result`) are copied from branch `research/claude-code-cli-headless` into the test fixtures. The tests check the parser, the acceptance criteria, failure classification and argv and environment assembly (allowlist, `Edit(//…)` rules). The real Claude Code never runs in CI.

### Modules tested

| Module | Seam |
|---|---|
| `workflow` (parse, validation, freeze) | `Project` (create_run with valid and invalid synthetic definitions) |
| `drive` / `decide` | `decide` (tables) and `Project` |
| `store` | `Project` (checkpoints, fault points, invariants) |
| `artifacts`, `context` | `Project` |
| `git`, `operations` | `Project` (three Git scenarios, real repository) |
| `reconcile`, `cleanup` | `Project` (fault points, `kill` smoke) |
| `runtime::claude_code` | fixtures |
| `runtime::fake` | indirectly, through all the others |
| `platform` | `Project` on the three OSes (locks, process identity, safe removal) |
| `surt` crate (commands, render, exit) | binary |

### Prior art

- The code has no tests yet; the only crate is the throwaway `ferris-says`. The reference is the plan: the test strategy in [ticket 16](../surt-architecture/issues/16-test-strategy.md), the real streams and the validated launcher in [ticket 19](../surt-architecture/issues/19-real-claude-code-stream-fixture.md) (branch `research/claude-code-cli-headless`), the plan's SQL and TOML (which already run in `sqlite3` and parse) and the terminal mocks in A, G, I and J, which are the message contract.

### CI

- `.github/workflows/ci.yml` with a macOS/Linux/Windows matrix. Each OS runs `cargo fmt --all -- --check`, `cargo clippy --all-targets --features test-support -- -D warnings` and `cargo test --features test-support`, with Git ≥ 2.38 pinned per runner.
- The smoke with the real Claude Code is manual, macOS only, following the checklist `docs/smoke-claude-code.md` (to be written in S6): in a toy repository, a Feature with two Tasks reaches `succeeded`.

## Out of Scope

- **M2 commands and flags:** `stop` (Ctrl-C covers it), `inspect`, `logs` (`status` shows the log path), `restore` (deleting the file recreates it), `delete`, `--new-run`, `--json`, `--run`, `--iter`, `--file`, TTY footer, 5-minute line without a TTY, ASCII symbols.
- **M3 mechanisms:** Transient Failure pause and backoff (in M1 it blocks), agent slots across processes (in M1 a per-Driver semaphore), global `~/.surt/config.toml`.
- **M4 mechanisms:** `merge_into_base = "ff-only"`, the `merge` step/`git.merge_into_base` and `--merge`/`--no-merge`; `on_success = "keep"`; overrides in `.surt/agents/`.
- **Real smoke on Linux and Windows** and API key mode (`--bare`): no manual validation in M1. Whatever CI misses becomes a bug.
- **Tests outside v1:** every row of the recovery tables, double Ctrl-C, concurrent migrations, a Git version matrix, ASCII without UTF-8, snapshots of every message, the date rolling over in the slug.
- **Outside the whole map:** a graphical interface; a daemon and `--detach`; external integrations (GitHub, Linear, Jira, push, PR); a second real AI runtime; project-specific workflows or overrides of the built-in ones; cancelling a single Task with partial delivery; coordinating rate limits between Drivers; secret redaction and log rotation; removing each Task's worktree right after integrating it; cloud infra.

## Further Notes

- No M1 cut requires a migration: `0001_initial.sql` already has `cleaned_at`, `execution.slot`, `failure_class`, `ref_move` and the stop/cancel request columns. M2–M4 land without changing the schema.
- **M1 done:** (1) green suite in CI on the three OSes, with the E2E under 60 s per OS; (2) the manual smoke with the real Claude Code on macOS passing.
- For implementers, the reference is the plan: [A–L](../../docs/architecture/), [ADRs 0001–0012](../../docs/adr/), the vocabulary in [`CONTEXT.md`](../../CONTEXT.md) and the rules in [`rules/`](../../rules/). A deliberate deviation from a rule is recorded in the rules' `Exceptions:` format, as [D](../../docs/architecture/D-database.md#rule-exceptions) and ADR 0012 already do.
- The schemas in `docs/architecture/schemas/` move to the crate that owns them: the Operation schemas to `surt-engine` and the agent schemas to the corresponding Agent in `surt`. `docs/architecture/migrations/0001_initial.sql` is copied unchanged.
- S6 can run in parallel with S4/S5 as soon as S3 fixes the `AgentRuntime` trait.
- After M1: M2 Operate and debug → M3 Run unattended → M4 Fit the project. v1 is the end of M4.
