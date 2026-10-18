# J. CLI Surface

The commands of v1 ([K](K-roadmap.md) lists the subset Milestone 1 ships), how they refer to things, what they print, their exit codes and the configuration they read. The engine is generic; these commands are the built-in feature workflow's front end and are the only layer that knows its phases. Vocabulary is defined in [`CONTEXT.md`](../../CONTEXT.md). Decision ticket: [CLI surface and UX](../../.scratch/surt-architecture/issues/14-cli-surface-and-ux.md).

## Commands

```
surt init
surt feature "<description>" [--name <slug>] [--base <ref>] [--merge | --no-merge] [--new]
surt feature <feature> [--new-run]
surt plan    <feature>
surt start   <feature>
surt approve <feature> [--rev N]
surt reject  <feature> -m "<reason>" [--rev N]
surt stop    <feature>
surt cancel  <feature> [--yes]
surt status  [<feature>] [--all] [--run N] [--json]
surt inspect <feature> [<task>] [--run N] [--json]
surt inspect <execution-id> [--json]
surt logs    <feature> [<task>] [--run N] [-f] [--raw]
surt logs    <execution-id> [-f] [--raw]
surt show    <feature> [<artifact>] [--rev N] [--task <key> [--iter N]] [--file <path>] [--run N] [--json]
surt restore <feature> <artifact> [--yes]
surt delete  <feature>
```

| Command | Drives? | What it does |
|---|---|---|
| `init` | no | Creates `.surt/` in the project ([below](#surt-init-and-finding-the-project)). |
| `feature "<description>"` | yes | Creates a Feature and its first run, drives to the spec Approval Gate. |
| `feature <feature>` | yes | Resumes the spec phase of the active run; with `--new-run`, starts a new run of the Subject. |
| `plan` | yes | Drives the Task Agent to the task-manifest Approval Gate. Requires the spec approved. |
| `start` | yes | Drives implementation to the Shipper's delivery and the opt-in fast-forward. Requires the manifest approved. |
| `approve` / `reject` | **no** | Record an Approval on the waiting gate's latest Revision and return. |
| `stop` | no | Asks the live Driver to stop (an Interruption). The run stays resumable. |
| `cancel` | no | Asks the live Driver to cancel, or cancels directly when no Driver is live. |
| `status`, `inspect`, `show`, `logs` | no | Read only ([Reading never writes](#reading-never-writes)). |
| `restore` | no | Rewrites a Working Copy from the latest Revision, discarding an unrecorded edit. |
| `delete` | no | Purges a Feature: worktrees, refs (except delivered Integration Branches), logs and history ([I. Cleanup](I-cleanup.md#surt-delete)). |

The brief's `resume <workflow>` is "run the phase command again" ([A. Process Model](A-process-model.md#commands-of-the-built-in-feature-workflow)). Its `stop <workflow>` is `surt stop`. Its `workflow delete <id>` is `surt delete`.

## Referring to features, runs, tasks and Executions

A `<feature>` argument is resolved in this order:

1. the exact full name (`20261017-add-login`);
2. the slug without its date (`add-login`), if exactly one Feature has it;
3. a prefix of the full name (`20261017-add`), if exactly one Feature matches.

Ambiguity refuses and lists the candidates. No command ever infers a "current" Feature from an omitted argument.

```
$ surt approve add-login
✖ "add-login" matches 2 features:
    20261015-add-login        AWAITING APPROVAL (spec rev 1)
    20261017-add-login        RUNNING
  use the full name
```

- **Run.** Commands that write always act on the Subject's active run. Read commands use the active run, or the latest one when none is active. `--run N` selects run `N` (`workflow_run.ordinal`). A run `N` the Subject doesn't have is refused (exit `3`), listing the Subject's runs with their derived states.
- **Task.** By item key (`B`). A task argument selects that item's Child Workflow Run.
- **Execution.** `inspect` and `logs` also accept an Execution id, or an unambiguous prefix of at least 8 characters (case-insensitive), which is how ids appear in Salvage Refs and Execution directories. The argument is tried as a `<feature>` first; only when no Feature matches and it is made of hex digits and `-` is it resolved as an Execution, so `surt inspect <id>` takes no `<task>` or `--run`. A prefix shorter than 8 characters, or one several Executions share, refuses (exit `3`) and lists up to 10 candidates with their Feature, run, Task, step, Iteration, Attempt and state; one no Execution has refuses as `no feature or execution matches`.
- **Surt prints Execution ids whole.** Execution ids are UUID v7: their leading characters are a millisecond clock, so the Executions of one run usually share their first 8 to 12 characters and a short prefix is ambiguous. Surt's output (timelines, the Execution detail, its `logs:` line), Execution directories, log directories and Salvage Refs therefore always carry the full id, which resolves no matter how many Executions exist. A prefix is only ever something a user types.
- **Surt's own output** always prints full Feature names, so every suggested command can be copied as is.

`surt feature` is the exception to rule 2 and 3, because its argument may also be a description (see [below](#surt-feature-create-resume-or-start-again)).

## `surt init` and finding the project

```
$ surt init
✔ initialized surt in /Users/you/app/.surt
  .surt/.gitignore    (only config.toml and agents/ are versioned)
  .surt/config.toml   (defaults, commented)
  .surt/surt.db
  git 2.47.0 ✔  identity you@example.com ✔
  next: surt feature "<what you want to build>"
```

- **Explicit.** Every other command refuses outside an initialized project (exit `3`); none creates `.surt/` on its own:
  ```
  ✖ not a surt project: no .surt/ in /Users/you/app
    run: surt init
  ```
- **Project root** is the toplevel of the repository's main worktree. `init` from a subdirectory creates `.surt/` at the toplevel and says so. Other commands walk up from the current directory to the toplevel. A command run inside a Surt-managed worktree (`~/.surt/worktrees/…`) refuses.
- **Preflight.** `init` runs the same Git preflight as the driving commands (Git ≥ 2.38, a user identity; [A. Git Integration](A-git-integration.md)) and refuses if it fails.
- **Idempotent.** Running it again reports what exists. It never overwrites `config.toml` or `agents/`, and it rewrites `.surt/.gitignore` only if it is missing.
- **Built-in agents are not materialized.** Their defaults live in the binary. A local override under `.surt/agents/<id>/` is written by hand, field by field ([F. Agent Contract](F-agent-contract.md#built-in-defaults-and-local-overrides)). There is no `surt agent export` in v1.

## `surt feature`: create, resume or start again

1. **Reference or description.** The argument is a reference only when it is **exactly** the full name of an existing Feature. Anything else is a description, from which the name is derived ([H. Subject naming](H-artifact-storage.md#subject-naming)).
2. **Re-running a description resumes nothing and duplicates nothing.** If the description's slug equals the slug of a Feature with an **active run**, ignoring the date prefix and any `-N` suffix, the command refuses instead of creating `-2`. This holds after midnight too, when the date prefix would differ:
   ```
   $ surt feature "add login"          # ↑ Enter after a Ctrl-C
   ✖ 20261017-add-login already has an active run for "add login" (INTERRUPTED)
     resume it:       surt feature 20261017-add-login
     or create anew:  surt feature "add login" --new
   ```
   `--new` skips this guard. Without an active run of the same slug, the `-N` suffix stays automatic.
3. **Resuming.** `surt feature <full-name>` drives the active run up to the spec Approval Gate, regenerating after a rejection. When the spec is already approved there is nothing to do in this phase:
   ```
   $ surt feature 20261017-add-login
   ✔ spec rev 2 already approved — next: surt plan 20261017-add-login
   ```
   It exits `0` and changes nothing.
4. **A new run of the same Subject** needs `--new-run`. Without it, a Subject with no active run refuses:
   ```
   $ surt feature 20261017-add-login
   ✖ 20261017-add-login has no active run (run 1 cancelled at 15:02)
     start run 2 from the same request: surt feature 20261017-add-login --new-run

   $ surt feature 20261017-add-login --new-run
   ✔ run 2 of 20261017-add-login   base main @ b22e310   branch surt/20261017-add-login.2
   ```
   It refuses with exit `3` while the Working Copy holds an unrecorded edit left by the previous run ([I](I-cleanup.md#unrecorded-working-copy-edits)). Run `N+1` takes the Subject's latest `request` Revision as a new human-origin Initial Artifact. Its Base is the current `HEAD`, or `--base`. It freezes the definitions, agents and configuration again. A `succeeded` Subject works the same way.

`--base <ref>` resolves `<ref>` to a commit when the run is created; the default is `HEAD`. `--merge` and `--no-merge` override `[git] merge_into_base` for this run only, as `ff-only` and `never`, before the configuration is frozen. They exist on no other command.

## Approve and reject

`approve` and `reject` only record ([A](A-process-model.md#commands-of-the-built-in-feature-workflow)). They scan the Working Copy first, so an edit becomes the Revision they target ([H](H-artifact-storage.md#what-freezing-means-at-approval)). `--rev N` refuses if the latest Revision is not `N`. Both end with the command that continues:

```
$ surt approve add-login
✔ approved spec rev 2 (agent) sha256:3f9a2c… by you@example.com
  next: surt plan 20261017-add-login

$ surt reject add-login -m "falta o fluxo OAuth"
✔ rejected spec rev 1 — the Spec Agent will redo it reading your reason
  regenerate: surt feature 20261017-add-login

$ surt reject add-login -m "task C grande demais"
✔ rejected manifest rev 1 (with tasks/A.md … tasks/D.md)
  regenerate: surt plan 20261017-add-login
```

### Next-command hints

The engine does not know which commands exist, and the workflow schema carries no UX text. The feature commands own one fixed table, next to the preconditions they already check:

| Waiting or just decided gate | After `approve` | After `reject`, and to resume the phase |
|---|---|---|
| `approve-spec` | `surt plan <feature>` | `surt feature <feature>` |
| `approve-tasks` | `surt start <feature>` | `surt plan <feature>` |

Blocked steps, interruptions and refusals point at the command of the phase they happened in.

## Stop and cancel

Both go through SQLite, never through signals between Surt processes ([A. External control](A-process-model.md#external-control)).

**`surt stop`** writes a stop request on the root run. The live Driver reads it on its next tick and performs the same Interruption as a first Ctrl-C: agent Executions stop gracefully, a running Operation finishes, and the Driver exits `130`.

```
T2 $ surt stop 20261017-add-login
⏳ stop requested; driver pid 48213 is stopping 3 executions…
✔ stopped. resumable: surt start 20261017-add-login
T1   ⏳ stop requested from another terminal; stopping…
T1   ⏹ task B build attempt 1 interrupted
T1   ⏹ stopped by surt stop (from another terminal)
T1     resume: surt start 20261017-add-login
```

- A stop request addresses the Driver that was live when it was written. A driving command clears any leftover stop request when it takes the run lock, so a request whose Driver died first never stops the next one.
- With no live Driver there is nothing to stop. It refuses with exit `3` and shows the run's derived state.
- `stop` asks no confirmation: the run stays resumable.
- `stop` waits for the Driver to release the run lock, the same way `cancel` waits for a live Driver. **The wait has no timeout**: the Driver's own grace period bounds it. A Ctrl-C on `surt stop` leaves the request in place, and the Driver still stops.
- Stopped Executions end `interrupted` and spend no Attempt, the same as a Ctrl-C.

**`surt cancel`** confirms first ([Confirmations](#confirmations)), then behaves as in [A](A-process-model.md#external-control) and [I](I-recovery.md#without-a-driver-surt-cancel-and-surt-status). A Driver that is cancelled from another terminal exits `5`.

## Reading never writes

`status`, `inspect`, `show` and `logs` never write to SQLite, the Working Copy or the disk. They never take the run lock and never recover ([I](I-recovery.md#without-a-driver-surt-cancel-and-surt-status)). They are safe in `watch surt status`, in scripts and on a read-only checkout.

- They compare the Working Copy with its baselines and **show** an unrecorded edit, but only a command that acts (`approve`, `reject`, a driving command) records it as a Revision ([H. Edit detection](H-artifact-storage.md#edit-detection)).
- A missing Working Copy file is shown, not recreated.

### `surt status`

Without an argument, one line per Feature with an active run. `--all` adds terminal runs.

```
$ surt status
FEATURE                 RUN  STATE              DRIVER     PROGRESS
20261017-add-login      1    RUNNING            pid 48213  2/4 merged · 1 running · 1 queued
20261016-fix-cache      1    AWAITING APPROVAL  —          manifest rev 1 (3 tasks)
20261015-refactor-db    2    BLOCKED            —          task C: 3 attempts failed
agent slots: 3/3 in use
```

With a Feature, the detail. States are derived as in [G. Workflow Run](G-state-machines.md#workflow-run).

```
$ surt status add-login
20261017-add-login · run 1   RUNNING   driver pid 48213 since 14:02
  base     main @ a11c9d0
  branch   surt/20261017-add-login @ 3c1aa20 · merge into base: never
  spec     rev 2 (agent) ✔ approved by you@example.com 13:10
  tasks    manifest rev 1 (agent) ✔ approved 13:40 · 4 tasks
  implement
    A  ✔ merged    iter 1 · as 3c1aa20
    B  ● build     iter 2 · attempt 1 · 6m
    C  ⏳ queued    build iter 1 · waiting for agent slot (3/3 in use)
    D  ⏸ pending   waits for B
```

When the Feature has several runs, the headline names the one it shows and how many exist: `20261017-add-login · run 2 of 2   AWAITING APPROVAL`. `--run 1` shows the earlier one.

At an Approval Gate it lists everything the Approval covers. For an Item List that is the list and every file it references ([F](F-agent-contract.md)):

```
$ surt status fix-cache
20261016-fix-cache · run 1   AWAITING APPROVAL   (no driver)
  tasks  manifest rev 1 (agent) — approving covers the manifest and all 3 task files:
           .surt/features/20261016-fix-cache/manifest.json
           .surt/features/20261016-fix-cache/tasks/{A,B,C}.md
         ✎ tasks/B.md edited — recorded as rev 2 on approve (approval then targets rev 2)
  approve: surt approve 20261016-fix-cache    reject: surt reject 20261016-fix-cache -m "…"
```

Blocked Tasks show the blocked step, its last error and the command to resume ([G](G-state-machines.md#workflow-run)). Integration shows the merged SHA or the Conflict reason (`merge: src/auth.rs`, `checks: test`). Recovery-related lines (`⟳ integrating … (pending)`, an agent running unattended) are in [I](I-recovery.md). Terminal runs show `cleanup pending` or `kept` ([I. Cleanup](I-cleanup.md#in-surt-status)).

### `surt inspect`

Timelines read from `event` ([D. Events](D-database.md#events)) and the Execution rows, at three levels:

```
$ surt inspect add-login            # run: iterations, gates, Driver, recovery
13:02:40  run.created         base main @ a11c9d0 · by you@example.com
13:02:41  driver.started      pid 48213
13:09:12  revision.published  spec iter 1 · attempt 1 · spec rev 1 (agent)
13:20:05  approval.recorded   approve-spec iter 1 · spec rev 1 rejected — "falta o fluxo OAuth" · by you@example.com
13:20:06  iteration.started   spec-loop · iter 2 until approve-spec not met: rejected
14:02:30  run.created         task B

$ surt inspect add-login B          # one Task: steps, attempts, errors
14:02:11  iteration.started   cycle · iter 2 until integrate not met
14:02:11  step.running        build iter 2
14:02:11  execution.started   build iter 2 · attempt 1 · agent-0 · claude-code · 0192f3a8-7b1c-…
14:09:40  execution.failed    build iter 2 · attempt 1 · invalid_delivery — final JSON missing "outputs"

$ surt inspect 0192f3a8-7b1c-7000-8000-4a1f0c3e9b21     # one Execution (or a unique prefix)
execution 0192f3a8-7b1c-7000-8000-4a1f0c3e9b21
  feature   20261017-add-login · run 1 · task B
  step      build iter 2 · attempt 1 · agent-0
  role      builder
  state     failed (retryable) · invalid_delivery — final JSON missing "outputs"
  profile   claude-code · claude-sonnet-5-5 · effort high · dontAsk
  session   7c1e9d2a-5b1e-4c3f-9a0e-2f6d8b7c1a40 · model claude-sonnet-5-5 · version 2.1.0
  process   pid 50122 · exit 0 · 7m29s
  usage     12 turns · 1200 input · 300 output · 9000 cache read · $0.42
  denied    Write, Bash
  inputs    item B (manifest rev 1) sha256:85fb633c285c…
            spec rev 2 sha256:3f9a0c1d2e3f…
  worktree  /Users/dev/.surt/worktrees/surt-3fa91c/20261017-add-login/B
  salvage   refs/surt/salvage/0192f3a8-7b1c-7000-8000-4a1f0c3e9b21
  logs      surt logs 0192f3a8-7b1c-7000-8000-4a1f0c3e9b21
```

The Execution detail reads only SQLite (the Execution row, its `runtime_info`, its Inputs with their Revision numbers and identity SHA-256, the Task's `run_branch`) plus a read-only `git for-each-ref` for `refs/surt/salvage/<execution-id>`, so it works after cleanup and without log files. The profile is the frozen agent's: runtime, model, requested effort and, for Claude Code, permission mode. `session` and `usage` show what the runtime reported. A value the runtime did not report is left out, never printed as `0`, and a line with nothing to show is left out. An Operation's Execution shows `operation <id>` instead of a role and profile.

Each line is the event's local time (with its date when it is from an earlier day than today), its kind and its details: the Task (run timeline only), the step and Iteration, the Attempt, then the kind's `event.data` fields ([D. Event data](D-database.md#event-data)). A field that is missing, as in a Milestone 1 event's `{}`, is left out, so such an event renders as its bare kind. The run timeline holds the root run's events plus each Task's `run.*` events; a Task's timeline holds every event of its Child Workflow Run. `--run N` picks the run, an unknown Task refuses and lists the run's Tasks. Execution ids are printed whole so they can be passed to `surt inspect <execution-id>` and `surt logs`.

### `surt show`

```
$ surt show add-login                       # index of artifacts
ARTIFACT   REVS  LATEST
request    1     rev 1 (human)
spec       2     rev 2 (agent) ✔ approved
manifest   1     rev 1 (agent) ✔ approved · 4 files
delivery   —

$ surt show add-login manifest              # the Item List JSON → stdout
$ surt show add-login manifest --file tasks/B.md
$ surt show add-login feedback --task B     # an artifact of task B's Child Workflow Run
$ surt show add-login spec --rev 1 > old-spec.md
```

- Content goes to stdout byte for byte. The latest Revision is the default. `--rev N` selects one, and `--task <key> --iter N` selects the Revision produced in that Iteration of the Task. `--iter` without `--task`, or together with `--rev`, is invalid usage (exit `2`); an Iteration that produced no Revision of the artifact refuses and lists the Iterations that did.
- `--file <path>` prints one file of the selected Revision byte for byte instead of its primary file. An unknown path refuses and lists the files of that Revision.
- An unknown artifact name refuses and lists the existing ones.
- When the Working Copy holds an unrecorded edit, `show` prints the recorded Revision and warns on stderr: `⚠ Working Copy has an unrecorded edit to spec.md`.
- `--json` returns the metadata (number, origin, SHA-256, Approval) together with the content.
- `show` is how artifacts are read after cleanup removed the Working Copy.

### `surt logs`

`logs` renders the agents' streams from disk; there is no daemon to ask. Streams live in `.surt/logs/<subject>/<execution-id>/` and survive cleanup; only `surt delete` removes them ([I. Cleanup](I-cleanup.md#logs)). When a stream no longer exists, `logs` says so and points at `inspect`.

```
$ surt logs add-login -f            # every agent running in this Feature, multiplexed
B build#2 │ Read src/auth.rs
C build#1 │ Bash cargo test -p auth
B build#2 │ Edit src/auth.rs (+12 −3)
B build#2 │ "Added the OAuth callback; the expiry test is still missing…"
C build#1 │ ✔ delivered (commit 9e04b7d)
```

- **Rendered** by default: one line per tool call, the first line of each assistant message, no tool results. `--raw` prints the original `stream.jsonl` lines.
- **Without `-f`**: the latest Execution of each Task, or of the given Task.
- **`-f`** follows until the run's Driver stops, picking up new Executions (the next Attempt, the next Task) as they start.
- The `B build#2 │` prefix appears only when there is more than one source.

## Foreground progress

A driving command prints one line per relevant state change: a Step Run starting or finishing, a failure, a merge, a recovery or salvage warning, a pause. It never prints agent text; that is `surt logs`.

- **On a TTY**, a footer under those lines is redrawn in place:
  ```
  ✔ task A merged (3c1aa20)
  ↻ task B build attempt 2
  ── 2 running (B build 6m, C review 1m) · 1 queued · slots 3/3 · 32m ──
  ```
- The footer is drawn with `\r` plus ANSI erase-line, erased before each event line and redrawn after it, and erased before the final summary, so the scrollback holds only event lines. Durations read `42s`, `6m`, `2h05m`.
- **Without a TTY** there is no footer. After 5 minutes without an event it prints a line so CI does not assume a hang: `… still running: B build (15m), C review (3m)`.
- It ends with a summary and the next command, as in the mocks of sections A, G and I.

## Output channels and non-interactive use

- **stderr** carries progress, warnings and errors (`✔ ✖ ⚠ ↻ ⏸ ⛔`). **stdout** carries data only: the `status` table, `show` content, `logs` lines, `--json` documents. `surt show add-login spec > spec.md` writes only the spec.
- **`--json`** exists on `status`, `inspect` and `show`. Each prints one document with `"schema": 1`; additions keep the version, removals or renames bump it. Driving commands have no `--json` in v1: a script uses their exit code, then `surt status --json`.
  ```
  $ surt start add-login 2>progress.log; echo $?
  4
  $ surt status add-login --json | jq -r '.tasks[] | select(.state=="blocked") | .key'
  B
  ```
- **Without a TTY** nothing is ever asked. A command that would confirm refuses with exit `3` unless `--yes` is given. There are no colours, spinners or footers, only the same lines.
- `NO_COLOR` disables colour. When the terminal is not UTF-8 (an old Windows console), the symbols become `[ok] [error] [warn] [retry] [paused] [blocked]`. The two settings are independent.
  - The engine's `platform` module answers "does the console output UTF-8". Unix: the first non-empty of `LC_ALL`, `LC_CTYPE`, `LANG` must name UTF-8 (`UTF-8` or `utf8`, any case); none set counts as UTF-8. Windows: the console output code page must be `65001`; a process with no console, or whose stdout and stderr are both redirected, counts as UTF-8, since the bytes go to a pipe or file, not to the console.
  - Every glyph the CLI prints lives in one symbol table (`surt` crate, `symbols.rs`), which also holds its ASCII form. Lines are converted at the print boundary, so a glyph in an engine message (the `→` of a cycle) is converted too. `show` file content is never converted.

    | Glyph | ASCII | Glyph | ASCII | Glyph | ASCII |
    |---|---|---|---|---|---|
    | `✔` | `[ok]` | `⏹` | `[stopped]` | `─` | `-` |
    | `✖` | `[error]` | `●` | `*` | `…` | `...` |
    | `⚠` | `[warn]` | `⏳` | `[waiting]` | `−` | `-` |
    | `↻` | `[retry]` | `✎` | `[edited]` | `—` | `-` |
    | `⏸` | `[paused]` | `⟳` | `[integrating]` | `·` | `\|` |
    | `⛔` | `[blocked]` | `│` | `\|` | `→` | `->` |
- `SURT_LOG` (`tracing` filter syntax, e.g. `SURT_LOG=debug`) turns on Surt's own diagnostic logs on stderr, for any command.

## Exit codes

The same on macOS, Linux and Windows.

| Code | Meaning | Examples |
|---|---|---|
| `0` | The command did what it promises | reached its phase's gate; run `succeeded`; Approval recorded; nothing to do |
| `1` | Unexpected error | a bug, I/O failure, unreadable database |
| `2` | Invalid usage | unknown flag, missing argument (clap's default) |
| `3` | **Refused**, nothing changed | precondition not met; Driver already live; ambiguous reference; Integration Branch moved; Git too old; pinned Revision edited; confirmation needed without `--yes`; `stop` without a Driver |
| `4` | **Blocked**: stopped needing a human outside an Approval Gate | Attempt or Iteration limit; `merge_into_base_interrupted` |
| `5` | The run was **cancelled** while this command drove it | `surt cancel` from another terminal |
| `130` | **Interrupted**, resumable | Ctrl-C, SIGHUP, console closed, `surt stop` |

- `surt start` that ends with `⚠ not merged: main moved since base` exits `0`. The delivery exists; the warning is in the text and in `status --json`.
- When some Tasks are blocked and the rest merged, the command exits `4`.
- A refusal raised in the middle of driving (the pinned-Revision edit of [H](H-artifact-storage.md#revalidation)) exits `3` once running work has finished.

## Confirmations

| Command | Confirms? | Why |
|---|---|---|
| `cancel` | **yes**, showing the impact | it ends the run for good |
| `restore` | **yes**, showing the edit that is lost | it discards an unrecorded human edit |
| `delete` | **yes**, showing runs, refs, disk and history removed ([I](I-cleanup.md#surt-delete)) | it removes history |
| `stop`, `approve`, `reject` | no | resumable, or guarded by `--rev` |

```
$ surt cancel add-login
cancel 20261017-add-login run 1?
  2 running executions will be stopped · 2/4 tasks merged stay on surt/20261017-add-login
  worktrees are removed; task branches and salvage refs stay until surt delete
  starting over later needs a new run (surt feature 20261017-add-login --new-run)
[y/N] y

$ surt restore fix-cache manifest
discard your unrecorded edit to the manifest of 20261016-fix-cache?
  tasks/B.md  +4 −1
  tasks/C.md  new file (not referenced by any item)
[y/N]
```

`-y` / `--yes` skips the question. Without a TTY and without `--yes`, the command refuses with exit `3`.

## Error messages

Every error and refusal has the same shape. Errors go to stderr.

```
✖ <what failed or was refused, one line>
  <context: observed values, at most two lines>
  <what happened to the state, only when it is not obvious>
  <label>: <copyable command, full names>        (one to three ways out)
```

- Labels are short and fixed: `fix:`, `next:`, `resume:`, `regenerate:`, `undo:`, `inspect:`, `or:`.
- A refusal changes nothing, so it says nothing about state. A message prints a state line only when work did happen, as in `running work finished; nothing new was launched` ([H](H-artifact-storage.md#revalidation)).
- An unexpected error (exit `1`) prints `✖ internal error: <message>` and `rerun with SURT_LOG=debug for details`.

The wording of the refusals and warnings already drafted in sections A, G, H and I follows this shape and is part of this contract.

## Configuration

This is the single reference for configuration keys. Unknown keys are errors in every command.

### Project: `.surt/config.toml` (versioned)

```toml
# .surt/config.toml — project configuration (versioned; generated by surt init)
max_concurrent_agents = 3   # agent slots for this project; read live by every Driver
max_attempts = 3            # failed Attempts per Step Run before it blocks
max_iterations = 5          # Iterations per Loop without an Approval Gate before it blocks

[git]
merge_into_base = "never"   # "never" | "ff-only"

# [[checks]]
# name = "test"
# run = ["cargo", "test", "--workspace"]
# timeout = "10m"           # optional
```

| Key | Default | When it applies | Decided in |
|---|---|---|---|
| `max_concurrent_agents` | `3` | read live by every Driver before each launch | [A](A-process-model.md#agent-concurrency-limit) |
| `max_attempts` | `3` | frozen at root run creation | [G](G-state-machines.md#limits) |
| `max_iterations` | `5` | frozen at root run creation | [G](G-state-machines.md#limits) |
| `[git] merge_into_base` | `"never"` | frozen; `surt feature --merge`/`--no-merge` override it per run | [A. Git Integration](A-git-integration.md) |
| `[[checks]]` | none | frozen | [A. Git Integration](A-git-integration.md) |

The frozen keys and the per-run override form the `config` snapshot of [D](D-database.md#snapshots).

### Global: `~/.surt/config.toml` (optional)

On Windows the path is `%USERPROFILE%\.surt\config.toml`. `~/.surt/` also holds the worktrees ([A. Git Integration](A-git-integration.md)).

```toml
stop_grace_period = "10s"   # graceful stop before a forced kill
```

## Decided elsewhere

- Which command drives which phase, ownership refusals, Ctrl-C: [A. Process Model](A-process-model.md).
- Derived run states and the blocked lines: [G. State Machines](G-state-machines.md).
- Working Copy, edit detection, `--rev`, naming of Features: [H. Artifact Storage](H-artifact-storage.md).
- Recovery lines and refusals at start: [I. Recovery and Reconciliation](I-recovery.md).
- The stop and cancel request columns: [D. Database Schema](D-database.md#workflow-runs).
- What `delete` and cleanup remove, the cleanup lines of `status`, and the `--new-run` refusal for unrecorded edits: [I. Cleanup, Delete and Logs](I-cleanup.md).
- Which streams `logs` can read: [I. Cleanup](I-cleanup.md#logs).
