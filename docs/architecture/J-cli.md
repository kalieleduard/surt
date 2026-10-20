# J. CLI Surface

The commands of v1 ([K](K-roadmap.md) lists the subset Milestone 1 ships), how they refer to things, what they print, their exit codes and the configuration they read. The engine is generic and so are these commands: `surt run` creates a run of any workflow under `.surt/workflows/`, `surt continue` drives it again, and neither knows the workflow's steps or gates. The feature workflow is files under `.surt/`, written by `surt init --template feature`. Vocabulary is defined in [`CONTEXT.md`](../../CONTEXT.md). Decision ticket: [CLI surface and UX](../../.scratch/surt-architecture/issues/14-cli-surface-and-ux.md).

## Commands

```
surt init [--template <name>]
surt run <workflow> [--input <name>=<value> | --input <name>=@<path>]... [--name <slug>] [--new] [--merge | --no-merge]
surt continue <subject> [--new-run [--input <name>=<value> | --input <name>=@<path>]... [--merge | --no-merge]]
surt approve <subject> [--rev N]
surt reject  <subject> -m "<reason>" [--rev N]
surt answer  <subject> [--accept | --file <path> | -m <text>]
surt stop    <subject>
surt cancel  <subject> [--yes]
surt status  [<subject>] [--all] [--run N] [--json]
surt inspect <subject> [<item>] [--run N] [--json]
surt inspect <execution-id> [--json]
surt logs    <subject> [<item>] [--run N] [-f] [--raw]
surt logs    <execution-id> [-f] [--raw]
surt show    <subject> [<artifact>] [--rev N] [--item <key> [--iter N]] [--file <path>] [--run N] [--json]
surt restore <subject> <artifact> [--yes]
surt delete  <subject>
```

| Command | Drives? | What it does |
|---|---|---|
| `init` | no | Creates `.surt/` in the project and, with `--template`, writes a Template's workflows and Agents into it ([below](#surt-init-and-finding-the-project)). |
| `run <workflow>` | yes | Creates a Subject and the first run of `.surt/workflows/<workflow>.toml` from its declared inputs, drives to the first Approval Gate or the end ([below](#surt-run-create-a-run-and-drive-it)). |
| `continue <subject>` | yes | Drives the active run again to the next Approval Gate or the end; with `--new-run`, starts the next run of the Subject ([below](#surt-continue-drive-again-or-start-the-next-run)). |
| `approve` / `reject` | **no** | Record an Approval on the waiting gate's latest Revision and return. |
| `answer` | yes | Records the answers to the waiting input step's questions, then drives on like `continue` ([below](#surt-answer-answer-the-questions)). |
| `stop` | no | Asks the live Driver to stop (an Interruption). The run stays resumable. |
| `cancel` | no | Asks the live Driver to cancel, or cancels directly when no Driver is live. |
| `status`, `inspect`, `show`, `logs` | no | Read only ([Reading never writes](#reading-never-writes)). |
| `restore` | no | Rewrites a Working Copy from the latest Revision, discarding an unrecorded edit. |
| `delete` | no | Purges a Subject: worktrees, refs (except delivered Integration Branches), logs and history ([I. Cleanup](I-cleanup.md#surt-delete)). |

The brief's `resume <workflow>` is `surt continue <subject>` ([A. Process Model](A-process-model.md#commands)). Its `stop <workflow>` is `surt stop`. Its `workflow delete <id>` is `surt delete`.

## Referring to subjects, runs, items and Executions

A `<subject>` argument is resolved in this order:

1. the exact full name (`20261017-add-login`);
2. the slug without its date (`add-login`), if exactly one Subject has it;
3. a prefix of the full name (`20261017-add`), if exactly one Subject matches.

Ambiguity refuses and lists the candidates. No command ever infers a "current" Subject from an omitted argument.

```
$ surt approve add-login
✖ "add-login" matches 2 subjects:
    20261015-add-login        AWAITING APPROVAL (spec rev 1)
    20261017-add-login        RUNNING
  use the full name
```

- **Run.** Commands that write always act on the Subject's active run. Read commands use the active run, or the latest one when none is active. `--run N` selects run `N` (`workflow_run.ordinal`). A run `N` the Subject doesn't have is refused (exit `3`), listing the Subject's runs with their derived states.
- **Item.** By item key (`B`): the `[<item>]` argument of `inspect` and `logs`, and `--item` on `show`. It selects that item's Child Workflow Run.
- **Execution.** `inspect` and `logs` also accept an Execution id, or an unambiguous prefix of at least 8 characters (case-insensitive), which is how ids appear in Salvage Refs and Execution directories. The argument is tried as a `<subject>` first; only when no Subject matches and it is made of hex digits and `-` is it resolved as an Execution, so `surt inspect <id>` takes no `<item>` or `--run`. A prefix shorter than 8 characters, or one several Executions share, refuses (exit `3`) and lists up to 10 candidates with their Subject, run, item, step, Iteration, Attempt and state; one no Execution has refuses as `no subject or execution matches`.
- **Surt prints Execution ids whole.** Execution ids are UUID v7: their leading characters are a millisecond clock, so the Executions of one run usually share their first 8 to 12 characters and a short prefix is ambiguous. Surt's output (timelines, the Execution detail, its `logs:` line), Execution directories, log directories and Salvage Refs therefore always carry the full id, which resolves no matter how many Executions exist. A prefix is only ever something a user types.
- **Surt's own output** always prints full Subject names, so every suggested command can be copied as is.

## `surt init` and finding the project

```
$ surt init --template feature
✔ initialized surt in /Users/you/app/.surt
  .surt/.gitignore                                         created
  .surt/config.toml                                        created
  .surt/surt.db                                            created
  .surt/workflows/feature.toml                             created
  .surt/workflows/task-cycle.toml                          created
  .surt/agents/spec/agent.toml                             created
  …
  .surt/agents/shipper/schemas/base.schema.json            created
  git 2.47.0 ✔  identity you@example.com ✔
  next: surt run feature --input request="<what you want to build>"
```

The `next:` line names the Template's first run when `--template` wrote (or kept) its files; plain `surt init` prints `next: add a workflow under .surt/workflows/`.

- **Explicit.** Every other command refuses outside an initialized project (exit `3`); none creates `.surt/` on its own:
  ```
  ✖ not a surt project: no .surt/ in /Users/you/app
    run: surt init
  ```
- **Project root** is the toplevel of the repository's main worktree. `init` from a subdirectory creates `.surt/` at the toplevel and says so. Other commands walk up from the current directory to the toplevel. A command run inside a Surt-managed worktree (`~/.surt/worktrees/…`) refuses.
- **Preflight.** `init` runs the same Git preflight as the driving commands (Git ≥ 2.38, a user identity; [A. Git Integration](A-git-integration.md)) and refuses if it fails.
- **Templates.** `--template <name>` also writes a Template's Workflow Definitions and Agents into `.surt/workflows/` and `.surt/agents/` ([E](E-workflow-schema.md#where-definitions-live), [F](F-agent-contract.md#definitions-in-the-project)). The binary knows one Template, `feature`. Plain `surt init` writes no workflow and no Agent, for a project that defines its own; a driving command in a project with no workflow refuses with `fix: surt init --template feature`. An unknown name refuses (exit `3`) before anything is written:
  ```
  $ surt init --template bugfix
  ✖ no template is named "bugfix"
    templates: feature
    fix: surt init --template feature
  ```
- **Idempotent.** Running it again lists every file with `created` or `kept`. It never overwrites a file that exists: `config.toml`, `.gitignore` and any Template file are written only when missing, so rerunning `surt init --template feature` restores a deleted file and keeps every edit. Once written, the Template's files are the project's; a Surt upgrade never changes them.

## `surt run`: create a run and drive it

```
$ surt run feature --input request="add login" --input base=main --merge
✔ created 20261019-add-login   workflow: feature   base: main @ a11c9d0
  merge into base: ff-only (--merge)
```

1. **Inputs.** `--input <name>=<value>` is repeatable and split at the first `=`; `<name>=@<path>` reads the value from a file, relative to the current directory (UTF-8 text). The workflow's `[inputs]` say what it takes ([E](E-workflow-schema.md#run-inputs-inputs-and-subject)); a workflow with no inputs runs with `surt run <workflow>`. A missing `name=` is a usage error (exit `2`).
2. **Input refusals** (exit `3`, nothing written) name the problem, list every declared input with its format, whether it is optional (and its default) and its description, and say how to fix it. They are the one refusal whose context is longer than two lines:
   ```
   $ surt run feature
   ✖ input "request" is required
     inputs of feature:
       request  text     required  what to build
       base     git_ref  optional  branch, tag or commit to start from (default HEAD)
     fix: pass --input request=<value>
   ```
   The other problems read `workflow feature has no input "colour"` (fix: drop it or declare it), `input "request" is given twice`, `input "request" is empty`, `input "settings" is not JSON: <parser message>` and `input "base": "no-such-branch" does not name a commit`; a `base` of `HEAD` in a repository without commits says so and suggests `git commit --allow-empty -m "first commit"`. An `@path` that cannot be read refuses with the path and the system's reason.
3. **Name.** The Subject's slug comes from `--name`, else from the value of the input `[subject] from` names, else from the workflow name, with the date prefix and `-N` suffix of [H. Subject naming](H-artifact-storage.md#subject-naming). A value with no portable character refuses with `fix: … --name <slug>`.
4. **Re-running resumes nothing and duplicates nothing.** If the slug equals the slug of a Subject with an **active run**, ignoring the date prefix and any `-N` suffix, the command refuses instead of creating `-2`. This holds after midnight too, when the date prefix would differ:
   ```
   $ surt run feature --input request="add login"          # ↑ Enter after a Ctrl-C
   ✖ 20261017-add-login already has an active run
     resume: surt continue 20261017-add-login
     or: surt run feature --input request="add login" --new
   ```
   `--new` skips this guard. Without an active run of the same slug, the `-N` suffix stays automatic.
5. **Creation line.** `created <subject>   workflow: <name>`, then `   <input>: <branch> @ <short sha>` (or the short sha alone when detached) for each `git_ref` input, then the `merge into base:` line below. The run is then driven like `surt continue` drives one.

## `surt continue`: drive again or start the next run

`surt continue <subject>` takes a reference to an existing Subject (an unknown reference refuses) and drives its active run, whatever step it is at. It is the one command that resumes; it knows nothing about the workflow's gates or phases.

1. **Driving again.** It finishes a cleanup the previous run left pending, then drives the active run until an Approval Gate waits, the run ends, a step blocks or a Ctrl-C interrupts. A gate that still waits ends the drive at once, printing the same `⏸ … awaiting approval` lines, exit `0`; there is no "approve the spec first" check. After a rejection the step that produced the artifact runs again; after an approval the run goes on past the gate:
   ```
   $ surt continue add-login
   ● plan-tasks started
   …
   ⏸ manifest rev 1 awaiting approval in .surt/subjects/20261017-add-login
     approve: surt approve 20261017-add-login
     reject: surt reject 20261017-add-login -m "<reason>"
   ```
   When the only thing to do is a pending cleanup (the Subject has no active run), it finishes it and exits `0`.
2. **A new run of the same Subject** needs `--new-run`. Without it, a Subject with no active run refuses:
   ```
   $ surt continue 20261017-add-login
   ✖ 20261017-add-login has no active run (run 1 cancelled at 15:02)
     start run 2 with the same inputs: surt continue 20261017-add-login --new-run

   $ surt continue 20261017-add-login --new-run --input base=develop
   ✔ run 2 of 20261017-add-login   base: develop @ b22e310   branch surt/20261017-add-login.2
   ```
   Run `N+1` uses the workflow the Subject's latest run used, read from `.surt/` again and frozen with the configuration. Its inputs come from the engine (`NewRun.carry = Carry::PreviousInputs`), so the CLI names none of them:
   - every declared input that is not `git_ref` and that the previous run had as an artifact is carried as a new human-origin Initial Artifact from its latest Revision, unless `--input` gives it again;
   - `git_ref` inputs are never carried: each is resolved again from `--input` or its `default`, so `base` follows the branch it names;
   - an input the workflow now requires and the previous run did not have must be given; the input refusal of [`surt run`](#surt-run-create-a-run-and-drive-it) lists it.
   
   The started line lists each `git_ref` input like the creation line of `surt run`. It refuses with exit `3` while the Working Copy holds an unrecorded edit left by the previous run ([I](I-cleanup.md#unrecorded-working-copy-edits)). A `succeeded` Subject works the same way. A cleanup the previous run left pending is finished first, under that run's lock: the command drives the previous root run, which cleans and returns at once because it is terminal, then creates run `N+1` ([I. Cleanup](I-cleanup.md#when-cleanup-runs)).
3. **`--input`** (repeatable, same forms as `surt run`) exists only with `--new-run`; on a plain `continue` it is a usage error (exit `2`).

`--merge` and `--no-merge` override `[git] merge_into_base` for this run only, as `ff-only` and `never`, before the configuration is frozen. They apply wherever a command creates a root run: `surt run` and `surt continue --new-run`. Together they are a usage error (exit `2`). They exist on no other command. The creation line names the frozen mode, and the flag when one chose it:

```
$ surt run feature --input request="add login" --merge
✔ created 20261017-add-login   workflow: feature   base: main @ a11c9d0
  merge into base: ff-only (--merge)
```

The `merge into base:` line follows the creation line, under `✔ created …` and under `✔ run N of …` alike. Without a flag it reads `merge into base: ff-only` when the configuration says so, and is absent for the default `never`.

On a plain `continue` either flag refuses with exit `3`, because the Subject's latest run, active or not, already froze its mode; choosing again needs a new run:

```
$ surt continue 20261017-add-login --no-merge
✖ run 1 of 20261017-add-login already froze merge_into_base = "ff-only"
  to choose again, start a new run: surt continue 20261017-add-login --new-run --no-merge
```

## Approve and reject

`approve` and `reject` only record ([A](A-process-model.md#commands)). They scan the Working Copy first, so an edit becomes the Revision they target ([H](H-artifact-storage.md#what-freezing-means-at-approval)). `--rev N` refuses if the latest Revision is not `N`. Both end with the command that continues:

```
$ surt approve add-login
✔ approved spec rev 2 (agent) sha256:3f9a2c… by you@example.com
  next: surt continue 20261017-add-login

$ surt reject add-login -m "falta o fluxo OAuth"
✔ rejected spec rev 1 — spec will redo it reading your reason
  regenerate: surt continue 20261017-add-login

$ surt reject add-login -m "task C grande demais"
✔ rejected manifest rev 1 — plan-tasks will redo it reading your reason
  regenerate: surt continue 20261017-add-login
```

The step named after a rejection is the one whose Agent declares the gate's artifact as an output (`Decided.producer`); when no Agent step of the gate's workflow does, the line says `the step that produced it`.

## `surt answer`: answer the questions

An Agent that asks delivers a `questions` artifact, and an input step ([E](E-workflow-schema.md), [G](G-state-machines.md)) waits for the developer as a gate does. A driving command that stops there prints the questions and exits `0`:

```
$ surt run feature --input request="add login"
✔ created 20261020-add-login   workflow: feature   base: main @ a11c9d0
● spec started
✔ spec-questions rev 1 published
✔ spec questions
? spec-questions rev 1 awaiting answers (round 1)
  1. Log in by e-mail only, or by username too?
     why: changes the users table and acceptance criteria 2–4
     options: e-mail only · e-mail or username
     recommended: e-mail only
  2. Does a session expire, and after how long?
     why: decides whether the Feature needs a refresh flow
     recommended: 7 days, no refresh
  answer: surt answer 20261020-add-login
```

The `?` block names the questions' artifact, its Revision and the round (the rounds already answered plus one), then every question in order with `why:`, `options:` when the answer is a closed set, and `recommended:`. Approval blocks are unchanged; a run waiting on both prints both.

`surt answer <subject>` answers the latest Revision of the waiting input step's questions in one of four ways. `--accept`, `--file` and `-m` are mutually exclusive (exit `2`).

- **On a terminal, without a flag**, it asks each question on stderr and reads one line from stdin; an empty line takes the recommendation. Input that ends before the last question records nothing and refuses (exit `3`).
  ```
  $ surt answer add-login
  1. Log in by e-mail only, or by username too?
     why: changes the users table and acceptance criteria 2–4
     options: e-mail only · e-mail or username
     [e-mail only]
  2. Does a session expire, and after how long?
     why: decides whether the Feature needs a refresh flow
     [7 days, no refresh] 30 days
  ✔ answered spec-questions rev 1 by you@example.com
  ● spec continued (iteration 2, same conversation)
  ✔ spec rev 1 published
  ✔ spec iter 2 done
  ⏸ spec rev 1 awaiting approval in .surt/subjects/20261020-add-login
    approve: surt approve 20261020-add-login
    reject: surt reject 20261020-add-login -m "<reason>"
  ```
- **`--accept`** takes every recommendation.
- **`--file <path>`** reads TOML with one `"<id>" = "<answer>"` per question, relative to the current directory. A file that cannot be read refuses (exit `3`) with the path and the system's reason; one that is not a table of strings refuses as `invalid answers in <path>:<line>` with the parser's message and `inspect: surt show <subject> <artifact> --rev N`.
- **`-m "<text>"`** takes every recommendation and records the text as the round's note.
- **Without a terminal and without a flag** it is a usage error (exit `2`), so a script never hangs: `error: there is no terminal to ask the questions on; answer with --accept, --file <path> or -m <text>`.

Every question needs exactly one answer. Answers that miss an id or name an unknown one refuse (exit `3`), listing both, and record nothing:

```
✖ the answers do not match the questions of spec-questions rev 1
  unanswered: expiry
  unknown: typo
  inspect: surt show 20261020-add-login spec-questions --rev 1
```

It answers the Revision it read or printed; if another round replaced it meanwhile, it refuses as a stale Revision. The actor is resolved as for `approve`. Once recorded it prints `✔ answered <artifact> rev N by <actor>` and drives the run exactly as `surt continue <subject>` does, with the same output and exit codes. With no input step waiting it refuses (exit `3`) saying what the run waits for, and `approve` and `reject` refuse while only an input step waits:

```
$ surt answer add-login
✖ 20261020-add-login has no input step waiting (AWAITING APPROVAL)
  inspect: surt status 20261020-add-login

$ surt approve add-login
✖ 20261020-add-login has no approval gate waiting
  answer-spec is waiting for answers, not for approval
  next: surt answer 20261020-add-login
```

`surt show <subject> spec-questions` and `surt show <subject> spec-answers` print the rounds ([H](H-artifact-storage.md)).

### Next-command hints

The engine does not know which commands exist, and the workflow schema carries no UX text. There is no table per gate: every place that says how to go on prints `surt continue <subject>` (a waiting input step prints `surt answer <subject>` instead: `answer:` in the `?` block and in `status`, `next:` in the approve and reject refusals), namely `approve` (`next:`), `reject` (`regenerate:`), `status` (the `resume:` line and `resume` in `--json`), `stop` (`resumable:`), interrupted and blocked runs, an incomplete cleanup (`retry:`), the integration-branch refusals and `already has an active run`. Starting over says `surt continue <subject> --new-run` (the no-active-run refusal, the merge-flag refusal, the `cancel` question).

## Stop and cancel

Both go through SQLite, never through signals between Surt processes ([A. External control](A-process-model.md#external-control)).

**`surt stop`** writes a stop request on the root run. The live Driver reads it on its next tick and performs the same Interruption as a first Ctrl-C: agent Executions stop gracefully, a running Operation finishes, and the Driver exits `130`.

```
T2 $ surt stop 20261017-add-login
⏳ stop requested; driver pid 48213 is stopping 3 executions…
✔ stopped. resumable: surt continue 20261017-add-login
T1   ⏳ stop requested from another terminal; stopping…
T1   ⏹ item B build attempt 1 interrupted
T1   ⏹ stopped by surt stop (from another terminal)
T1     resume: surt continue 20261017-add-login
```

- A stop request addresses the Driver that was live when it was written. A driving command clears any leftover stop request when it takes the run lock, so a request whose Driver died first never stops the next one.
- With no live Driver there is nothing to stop. It refuses with exit `3` and shows the run's derived state.
- `stop` asks no confirmation: the run stays resumable.
- `stop` waits for the Driver to release the run lock, the same way `cancel` waits for a live Driver. **The wait has no timeout**: the Driver's own grace period bounds it. A Ctrl-C on `surt stop` leaves the request in place, and the Driver still stops.
- Stopped Executions end `interrupted` and spend no Attempt, the same as a Ctrl-C.

**`surt cancel`** confirms first ([Confirmations](#confirmations)), then behaves as in [A](A-process-model.md#external-control) and [I](I-recovery.md#without-a-driver-surt-cancel-and-surt-status). A Driver that is cancelled from another terminal exits `5`.

## `surt restore`

`surt restore <subject> <artifact>` discards an unrecorded Working Copy edit ([H](H-artifact-storage.md#editing-an-item-list)).

- It acts only on the Subject's **active** run. It takes that root run's lock with `try_lock` before showing anything; a live Driver refuses with exit `3`, naming its pid, without asking. Under the lock it first finishes pending Working Copy writes, so the impact it shows is what the restore does. A terminal run refuses with I's `fix: move or delete the file` ([I. Cleanup](I-cleanup.md#unrecorded-working-copy-edits)).
- It shows what will be lost, per file: changed with added and removed line counts, a new file (removed), or a missing file (recreated), then asks `[y/N]` ([Confirmations](#confirmations)). The line counts come from a small line-based count in the engine, not a diff library.
- When nothing differs it says so and exits `0`:
  ```
  $ surt restore 20261017-add-login spec
  ✔ spec already matches rev 1; nothing to restore
  ```
- An artifact without a Working Copy refuses and lists the artifacts that have one.

## Reading never writes

`status`, `inspect`, `show` and `logs` never write to SQLite, the Working Copy or the disk. They never take the run lock and never recover ([I](I-recovery.md#without-a-driver-surt-cancel-and-surt-status)). They are safe in `watch surt status`, in scripts and on a read-only checkout.

- They compare the Working Copy with its baselines and **show** an unrecorded edit, but only a command that acts (`approve`, `reject`, a driving command) records it as a Revision ([H. Edit detection](H-artifact-storage.md#edit-detection)).
- A missing Working Copy file is shown, not recreated.

### `surt status`

Without an argument, one line per Subject with an active run. `--all` adds terminal runs.

```
$ surt status
SUBJECT                 RUN  STATE              DRIVER     PROGRESS
20261017-add-login      1    RUNNING            pid 48213  2/4 merged · 1 running · 1 queued
20261016-fix-cache      1    AWAITING APPROVAL  —          manifest rev 1 (3 items)
20261015-refactor-db    2    BLOCKED            —          item C build blocked
agent slots: 3/3 in use
```

With a Subject, the detail. States are derived as in [G. Workflow Run](G-state-machines.md#workflow-run).

```
$ surt status add-login
20261017-add-login · run 1   RUNNING   driver pid 48213 since 14:02
  base     main @ a11c9d0
  branch   surt/20261017-add-login @ 3c1aa20 · merge into base: never
  spec     rev 2 (agent) ✔ approved by you@example.com 13:10
  manifest rev 1 (agent) ✔ approved 13:40 · 4 items
  implement
    A  ✔ merged    iter 1 · as 3c1aa20
    B  ● build     iter 2 · attempt 1 · 6m
    C  ⏳ queued    build iter 1 · waiting for agent slot (3/3 in use)
    D  ⏸ pending   waits for B
```

Gate rows are labelled by the gate's artifact. The items section is titled by the key of the frozen definition's `per_item` step (`implement` in the `feature` Template), one row per item key; an integrated item reads `merged`. An approved or rejected Item List says how many items it lists (`· 4 items`), and the overview's progress for a waiting one reads `manifest rev 1 (3 items)`.

`agent slots: N/M in use` and a queued item's `waiting for agent slot (N/M in use)` count the whole project, not one Subject: `N` is the number of running agent Executions holding a slot across every run whose Driver is live, so with two Subjects driven at once an item of one can wait for the other's agents. `M` is the current `max_concurrent_agents` in `.surt/config.toml`. The count is derived from SQLite; `status` never probes the slot lock files ([A](A-process-model.md#agent-concurrency-limit)), since a probe would briefly take a slot and could make a Driver skip a launch. `status --json`'s `agent_slots` carries the same numbers.

While a Transient Failure pauses agent launches ([I](I-recovery.md#transient-failures-network-and-rate-limits)), the headline stays `RUNNING` and one line under it names the error code and the reset time the Execution recorded in `runtime_info.resets_at`, or `retrying with backoff` when it recorded none. Items waiting to launch read `queued … waiting for agent launches to resume` in place of `waiting for agent slot (N/M in use)`. Such an item names the agent step it waits to launch: one not started yet, or the one whose Attempt failed `transient` (with that Attempt's number, since a Transient Failure spends none). A queued item whose next launch is not an agent step names no step:

```
$ surt status add-login
20261017-add-login · run 1   RUNNING   driver pid 48213 since 14:02
  ⏸ agent launches paused: rate_limit — resets 03:00
  base     main @ a11c9d0
  branch   surt/20261017-add-login @ 3c1aa20 · merge into base: never
  implement
    A  ✔ merged    iter 1 · as 3c1aa20
    B  ⏳ queued    build iter 1 · attempt 1 · waiting for agent launches to resume
```

The pause lives only in the Driver's memory, so `status` derives it on every read and nothing stores it: a run is paused when its Driver is live, its latest ended agent Execution failed `transient`, and no agent Execution started after it. A reset time no later than that failure counts as none, as it does for the Driver.

When the Subject has several runs, the headline names the one it shows and how many exist: `20261017-add-login · run 2 of 2   AWAITING APPROVAL`. `--run 1` shows the earlier one.

At an Approval Gate it lists everything the Approval covers. For an Item List that is the list and every file it references ([F](F-agent-contract.md)). The engine decides what is an Item List: the gate's artifact is one when the Agent that produces it declares the output `format = "item_list"`, and the counts come from that Revision's index (`RevisionSummary.item_list`: its items, and the distinct item files they reference), never from the number of files:

```
$ surt status fix-cache
20261016-fix-cache · run 1   AWAITING APPROVAL   (no driver)
  manifest rev 1 (agent) ⏸ awaiting approval — approving covers the manifest and all 3 item files:
           .surt/subjects/20261016-fix-cache/manifest.json
           .surt/subjects/20261016-fix-cache/tasks/{A,B,C}.md
         ✎ tasks/B.md edited — recorded as rev 2 on approve (approval then targets rev 2)
  approve: surt approve 20261016-fix-cache    reject: surt reject 20261016-fix-cache -m "…"
```

An input step waiting for answers puts the run in `AWAITING ANSWER` and shows its questions' Revision, the round and how to answer; an answered one keeps its row with the actor:

```
$ surt status add-login
20261020-add-login · run 1   AWAITING ANSWER   (no driver)
  base     main @ a11c9d0
  spec-questions rev 1 (agent) ✔ answered by you@example.com 13:05 (round 1)
  spec-questions rev 2 (agent) ⏸ awaiting answers (round 2)
  answer: surt answer 20261020-add-login
```

Blocked items show the blocked step, its last error and the command to resume, plus `inspect: surt inspect <execution>` and `logs: surt logs <execution>` for the last failed Execution ([G](G-state-machines.md#workflow-run)). Integration shows the merged SHA or the Conflict reason (`merge: src/auth.rs`, `checks: test`). Recovery-related lines (`⟳ integrating … (pending)`, an agent running unattended) are in [I](I-recovery.md). Terminal runs show `cleanup pending` or `kept` ([I. Cleanup](I-cleanup.md#in-surt-status)).

The `branch` line keeps the frozen `merge into base` mode. Once the `merge` step ended, a `merge` line above `cleanup` gives its result, read from the step's recorded `step.done` fields ([A](A-git-integration.md#merging-into-the-base-opt-in)): `merged into main (fast-forward a11c9d0 → 4e1b2c0)`, or `not merged:` with the reason in the same words as the line the driving command printed: `main moved since base (a11c9d0 → b22e310)` (the Base, then where the branch was found), `main no longer exists (base a11c9d0)`, `the base a11c9d0 is not a branch` or `main is checked out in /Users/you/surt with changes`. A run whose mode is `never` records `disabled` and shows no line. While the fast-forward's Intention is pending, the line is the recovery one, `⟳ merge   main: a11c9d0 → 4e1b2c0 (pending)`.

```
$ surt status add-login
20261017-add-login · run 1   SUCCEEDED
  base     main @ a11c9d0
  branch   surt/20261017-add-login @ 4e1b2c0 · merge into base: ff-only
  …
  merge    not merged: main moved since base (a11c9d0 → b22e310)
  cleanup  done
```

### `surt inspect`

Timelines read from `event` ([D. Events](D-database.md#events)) and the Execution rows, at three levels:

```
$ surt inspect add-login            # run: iterations, gates, Driver, recovery
13:02:40  run.created         base main @ a11c9d0 · by you@example.com
13:02:41  driver.started      pid 48213
13:09:12  revision.published  spec iter 1 · attempt 1 · spec rev 1 (agent)
13:20:05  approval.recorded   approve-spec iter 1 · spec rev 1 rejected — "falta o fluxo OAuth" · by you@example.com
13:20:06  iteration.started   spec-loop · iter 2 until approve-spec not met: rejected
14:02:30  run.created         item B

$ surt inspect add-login B          # one item: steps, attempts, errors
14:02:11  iteration.started   cycle · iter 2 until integrate not met
14:02:11  step.running        build iter 2
14:02:11  execution.started   build iter 2 · attempt 1 · agent-0 · claude-code · 0192f3a8-7b1c-…
14:09:40  execution.failed    build iter 2 · attempt 1 · invalid_delivery — final JSON missing "outputs"

$ surt inspect 0192f3a8-7b1c-7000-8000-4a1f0c3e9b21     # one Execution (or a unique prefix)
execution 0192f3a8-7b1c-7000-8000-4a1f0c3e9b21
  subject   20261017-add-login · run 1 · item B
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

The Execution detail reads only SQLite (the Execution row, its `runtime_info`, its Inputs with their Revision numbers and identity SHA-256, the item's `run_branch`) plus a read-only `git for-each-ref` for `refs/surt/salvage/<execution-id>`, so it works after cleanup and without log files. The profile is the frozen agent's: runtime, model, requested effort and, for Claude Code, permission mode. `session` and `usage` show what the runtime reported. A value the runtime did not report is left out, never printed as `0`, and a line with nothing to show is left out. An Operation's Execution shows `operation <id>` instead of a role and profile.

Each line is the event's local time (with its date when it is from an earlier day than today), its kind and its details: the item (run timeline only), the step and Iteration, the Attempt, then the kind's `event.data` fields ([D. Event data](D-database.md#event-data)). A field that is missing, as in a Milestone 1 event's `{}`, is left out, so such an event renders as its bare kind. The run timeline holds the root run's events plus each item's `run.*` events; an item's timeline holds every event of its Child Workflow Run. `--run N` picks the run, an unknown item refuses and lists the run's items. Execution ids are printed whole so they can be passed to `surt inspect <execution-id>` and `surt logs`. The pause reads `agents.paused  until 03:00 · rate_limit · 2 waiting` (with `backoff step N` when no reset time was reported, and the date when the end falls on another day) and `agents.resumed  after 2h05m00s`. The Execution detail of a rate-limited Execution shows `failed (transient) · rate_limit — … · resets 03:00` when the runtime reported a reset time.

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
$ surt show add-login feedback --item B     # an artifact of item B's Child Workflow Run
$ surt show add-login spec --rev 1 > old-spec.md
```

- Content goes to stdout byte for byte. The latest Revision is the default. `--rev N` selects one, and `--item <key> --iter N` selects the Revision produced in that Iteration of the item. `--iter` without `--item`, or together with `--rev`, is invalid usage (exit `2`); an Iteration that produced no Revision of the artifact refuses and lists the Iterations that did.
- `--file <path>` prints one file of the selected Revision byte for byte instead of its primary file. An unknown path refuses and lists the files of that Revision.
- An unknown artifact name refuses and lists the existing ones.
- When the Working Copy holds an unrecorded edit, `show` prints the recorded Revision and warns on stderr: `⚠ Working Copy has an unrecorded edit to spec.md`.
- `--json` returns the metadata (number, origin, SHA-256, Approval) together with the content.
- `show` is how artifacts are read after cleanup removed the Working Copy.

### `surt logs`

`logs` renders the agents' streams from disk; there is no daemon to ask. Streams live in `.surt/logs/<subject>/<execution-id>/` and survive cleanup; only `surt delete` removes them ([I. Cleanup](I-cleanup.md#logs)). When a stream no longer exists, `logs` says so and points at `inspect`.

```
$ surt logs add-login -f            # every agent running in this Subject, multiplexed
B build#2 │ Read src/auth.rs
C build#1 │ Bash cargo test -p auth
B build#2 │ Edit src/auth.rs (+12 −3)
B build#2 │ "Added the OAuth callback; the expiry test is still missing…"
C build#1 │ ✔ delivered (commit 9e04b7d)
```

- **Rendered** by default: one line per tool call (`<Tool> <main argument>`, a path relative to the agent's working directory, an edit's `(+added −removed)` lines), the first line of each assistant message in quotes (`…` when more follows), `✔ delivered` for the runtime's accepted structured output (with a non-default outcome and the commit), no tool results. Unknown event types, unknown fields and lines that are not JSON render nothing. `stderr.log` and an Operation's `output.log` (one `=== <check>: <command>` header per Check) pass through after the stream.
- The engine resolves the sources and gives each Execution's log directory path; the CLI opens and renders the files, since rendering is text and the engine stays text-free.
- **`--raw`** prints the original `stream.jsonl` lines untouched and nothing else, without prefix, so the output pipes to `jq`.
- **Without `-f`**: the latest Execution of the run's own steps and of each item, or of the given item, or the given Execution. An Operation that ran no process has no log and is skipped when picking the latest. Sources print one after another, run first, then items by key.
- **`-f`** follows until the run's Driver stops, picking up new Executions (the next Attempt, the next item) as they start. It polls the log files and the run's Execution list every 200 ms (no file watcher), prints each file's new complete lines, and once the Driver is gone drains every file to its end and exits `0`. With no live Driver at start it prints what exists, as without `-f`, and exits. An Execution target follows only that Execution.
- The `B build#2 │` prefix (item, step, `#`Attempt; the run's own steps have no item) appears only when more than one source is printed, padded to align. With `-f` it appears for a Subject or item target (the sources are not known in advance), padded to the longest label seen so far, and never for an Execution target.

## Foreground progress

A driving command prints one line per relevant state change: a Step Run starting or finishing, a failure, a merge, a recovery or salvage warning, a pause. It never prints agent text; that is `surt logs`.

- **On a TTY**, a footer under those lines is redrawn in place:
  ```
  ✔ item A merged (3c1aa20)
  ↻ item B build attempt 2
  ── 2 running (B build 6m, C review 1m) · 1 queued · slots 3/3 · 32m ──
  ```
- The footer is drawn with `\r` plus ANSI erase-line, erased before each event line and redrawn after it, and erased before the final summary, so the scrollback holds only event lines. A footer wider than the terminal is cut to one line ending in `…`, since erase-line clears only the physical line the cursor is on; the width comes from the console (falling back to `COLUMNS`, then 80). Durations read `42s`, `6m`, `2h05m`.
- `slots N/M` agrees with `surt status`: N counts every slot in use in the project, including this Driver's agents still running above a lowered `max_concurrent_agents` (`slots 3/1`), and M is the limit read for this tick.
- While agent launches are paused, the footer says until when and how many agents wait instead of the queue: `── 1 running (C review 1m) · ⏸ paused until 03:00 · 2 waiting · slots 1/3 · 32m ──`, or `paused until 13:01 (backoff 1m)` without a reset time.
- **Without a TTY** there is no footer. After 5 minutes without an event it prints a line so CI does not assume a hang: `… still running: B build (15m), C review (3m)`. A pause adds itself, and a pause with nothing running still prints: `… agent launches paused until 03:00 — 2 waiting`.
- A step of a Child Workflow Run is named `item <key> <step>` (`● item B build started`), and an integration `✔ item <key> merged (<sha>)`.
- An agent Execution that continues the conversation of an earlier one ([F](F-agent-contract.md)) starts with `● <step> continued (iteration I, same conversation)` instead of `● <step> started`. When the runtime reports that it could not continue and restarted fresh in the same Execution, a second line says why: `↻ <step> restarted (iteration I): transcript_missing` (or `continuation_refused`). A later Attempt keeps the retry line `↻ <step> iter I attempt N`, continued or not.
- It ends with a summary and the next command, as in the mocks of sections A, G and I. A run that succeeds with an Integration Branch first prints `✔ delivered — surt/<subject> @ <tip>`; a run without one prints only `✔ run succeeded`. The line names no step: the engine records which step moved the branch, not which one delivered.

## Output channels and non-interactive use

- **stderr** carries progress, warnings and errors (`✔ ✖ ⚠ ↻ ⏸ ⛔`). **stdout** carries data only: the `status` table, `show` content, `logs` lines, `--json` documents. `surt show add-login spec > spec.md` writes only the spec.
- **`--json`** exists on `status`, `inspect` and `show`. Each prints one document with `"schema": 1`; additions keep the version, removals or renames bump it. Driving commands have no `--json` in v1: a script uses their exit code, then `surt status --json`.
  ```
  $ surt continue add-login 2>progress.log; echo $?
  4
  $ surt status add-login --json | jq -r '.items[] | select(.state=="blocked") | .key'
  B
  ```
- **Without a TTY** nothing is ever asked. A command that would confirm refuses with exit `3` unless `--yes` is given. There are no colours, spinners or footers, only the same lines.
- A path inside the project is shown relative to its toplevel with `/` on every OS (`.surt/subjects/20261017-add-login/spec.md`), in text and in `--json`. A path outside it, such as a worktree under `~/.surt/`, is shown absolute in the OS's own form.
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
    The `?` of a waiting input step is ASCII already and prints as is.
- `SURT_LOG` (`tracing` filter syntax, e.g. `SURT_LOG=debug`) turns on Surt's own diagnostic logs on stderr, for any command.

### `--json` documents

Each document serialises the read view the human output renders (`RunStatus`, `Timeline`, `ExecutionDetail`, `Shown`, `ArtifactSummary`); there is no second query path. The rules:

- `"schema": 1` is the first field of every document. Within schema 1 fields are only **added**; removing or renaming a field, or changing its type, bumps the schema.
- Timestamps are RFC 3339 UTC strings (`2026-10-17T14:09:40Z`). Ids (Execution ids, SHAs, SHA-256 identities) are full strings, never shortened.
- A run's derived state uses the human table's uppercase names (`RUNNING`, `AWAITING APPROVAL`, `AWAITING ANSWER`, `BLOCKED`, `INTERRUPTED`, `SUCCEEDED`, `CANCELLED`); `state` joins them with ` + ` as the table does, `states` lists them. Other closed values are lowercase words (`blocked`, `approved`, `attempts_exhausted`).
- A value nobody reported is **absent**, not `null`: no `driver` without a live Driver, no `turns` or `cost_usd` when the runtime did not report them, no `approval` on an undecided Revision. Empty lists other than the top-level collections (`subjects`, `gates`, `items`, `events`, `files`, `inputs`, `artifacts`) are absent too.
- A refusal or error prints nothing on stdout; the `✖` message goes to stderr with the usual exit code.

`surt status <subject> --json` prints one Subject; `surt status [--all] --json` prints `{"schema": 1, "subjects": [...]}` with the same object per Subject (an empty list when none is active). A blocked item carries its reason, last error and the command to resume:

```json
{
  "schema": 1,
  "name": "20261017-add-login",
  "run": 1,
  "runs": 1,
  "state": "BLOCKED",
  "states": ["BLOCKED"],
  "base": {"sha": "a11c9d0e5f60718293a4b5c6d7e8f90112233445", "branch": "main"},
  "branch": {"name": "surt/20261017-add-login", "tip": "9e04b7d0c1a2b3c4d5e6f708192a3b4c5d6e7f80"},
  "merge_into_base": "never",
  "agent_slots": {"in_use": 0, "total": 3},
  "gates": [
    {
      "step": "approve-spec",
      "kind": "approval",
      "artifact": "spec",
      "status": "approved",
      "revision": {"number": 1, "origin": "agent", "sha256": "5d41402a…(64 hex)", "files": ["spec.md"]},
      "approval": {"decision": "approve", "actor": "dev@example.com", "decided_at": "2026-10-17T14:02:11Z"}
    }
  ],
  "item_step": "implement",
  "items": [
    {"key": "A", "state": "succeeded", "step": "integrate", "iteration": 1, "attempt": 1,
     "since": "2026-10-17T14:05:02Z", "merged_sha": "3c1aa2…(40 hex)"},
    {"key": "B", "state": "blocked", "step": "build", "iteration": 1, "attempt": 3,
     "since": "2026-10-17T14:09:40Z", "blocked_reason": "attempts_exhausted",
     "last_error": {"execution": "01a14f9b-c44f-7608-80bd-fe56114a4306", "attempt": 3, "code": "exit_code", "message": "the agent exited with code 1"},
     "resume": "surt continue 20261017-add-login"}
  ],
  "blocked": [
    {"step": "build", "item": "B", "iteration": 1, "reason": "attempts_exhausted",
     "last_error": {"execution": "01a14f9b-c44f-7608-80bd-fe56114a4306", "attempt": 3, "code": "exit_code", "message": "the agent exited with code 1"}}
  ],
  "resume": "surt continue 20261017-add-login"
}
```

Optional fields: `ended_at`, `driver` (`pid`, `since`), `paused` while a Transient Failure pauses agent launches (`code`, plus `resets_at` when the Execution recorded a reset time), `cleanup` (`state` `done`, `pending` or `kept`; `left` lists items with `item` and `reason`; `kept` lists `moved_branch` with `branch`, or `unrecorded_edit` or `foreign_path` with `path`); per gate `edited` and `missing`, `round` and, once answered, `answer` (`actor`, `answered_at`) on an input step, and per gate Revision `item_list` (`items` and `item_files`, when the gate's artifact is an Item List); `item_step`, the key of the definition's `per_item` step; per item `waits_for`, `conflict` (`kind` `merge` with `paths`, or `checks` with `failed`); per last error `resets_at`; `merge` once the `merge` step ended, as recorded: `{"outcome": "merged", "from": …, "to": …}`, or `{"outcome": "skipped", "reason": …}` with `reason` `disabled`, `no_base_branch`, `worktree_not_clean` (plus `worktree`, the worktree's path) or `base_moved` (plus `found`, the full SHA the branch was at, unless it no longer exists).

Each gate entry has a `kind`: `approval`, or `input` for an input step. An input entry's `artifact` is its questions, its `status` is `not_reached`, `waiting` or `answered`, and it carries the `round` it waits for or answered.

`agent_slots` is the project-wide count of [`surt status`](#surt-status), the same in every Subject's document, whether or not that Subject has a live Driver.

`surt inspect <subject> [<item>] --json` prints the timeline; each event keeps its `data` object as recorded ([D](D-database.md)):

```json
{
  "schema": 1,
  "subject": "20261017-add-login",
  "run": 1,
  "item": "B",
  "events": [
    {"at": "2026-10-17T14:09:40Z", "kind": "execution.failed", "item": "B", "step": "build",
     "iteration": 1, "attempt": 3, "execution": "01a14f9b-c44f-7608-80bd-fe56114a4306",
     "data": {"code": "exit_code", "message": "the agent exited with code 1"}}
  ]
}
```

`surt inspect <execution-id> --json` prints one Execution; `reported` holds only what the runtime reported:

```json
{
  "schema": 1,
  "execution": {
    "id": "01a14f9b-c44f-7608-80bd-fe56114a4306",
    "subject": "20261017-add-login",
    "run": 1,
    "item": "B",
    "step": "build",
    "iteration": 1,
    "attempt": 3,
    "performer": {"kind": "agent", "role": "builder", "runtime": "claude-code",
                  "model": "claude-sonnet-5-5", "effort": "high", "permission_mode": "dontAsk"},
    "state": "failed",
    "failure": {"class": "retryable", "code": "exit_code", "message": "the agent exited with code 1"},
    "slot": 1,
    "pid": 48213,
    "started_at": "2026-10-17T14:08:02Z",
    "finished_at": "2026-10-17T14:09:40Z",
    "reported": {"version": "2.1.3", "session_id": "…", "model": "claude-sonnet-5-5", "exit_code": 1,
                 "turns": 12, "usage": {"input_tokens": 5120, "output_tokens": 830}, "cost_usd": 0.42},
    "inputs": [{"name": "item", "artifact": "manifest", "revision": 1, "sha256": "…(64 hex)", "item": "B"}],
    "worktree": "/home/dev/.surt/worktrees/…/20261017-add-login/B",
    "salvage_ref": "refs/surt/salvage/01a14f9b-c44f-7608-80bd-fe56114a4306"
  }
}
```

A Check or other Operation has `"performer": {"kind": "operation", "id": "check"}`. Optional fields: `outcome`, `failure`, `slot`, `pid`, `started_at`, `finished_at`, `worktree`, `salvage_ref`, and in `reported` each of `version`, `session_id`, `model`, `exit_code`, `turns`, `usage`, `cost_usd`, `permission_denials`, `resets_at`.

`surt show <subject> <artifact> --json` prints one Revision with every file (only the one `--file` names). A file's content is a UTF-8 string, or `content_base64` (RFC 4648, padded) when its bytes aren't UTF-8:

```json
{
  "schema": 1,
  "artifact": "manifest",
  "revision": 1,
  "origin": "agent",
  "sha256": "…(64 hex)",
  "approval": {"decision": "approve", "actor": "dev@example.com", "decided_at": "2026-10-17T14:04:30Z"},
  "files": [
    {"path": "manifest.json", "content": "[{\"key\":\"A\",\"needs\":[]}]"},
    {"path": "tasks/A.md", "content": "# Task A\n"},
    {"path": "tasks/logo.bin", "content_base64": "//4AQQ=="}
  ]
}
```

`edited` lists files whose Working Copy holds an unrecorded edit (the `⚠` warning still goes to stderr). `surt show <subject> --json` prints `{"schema": 1, "artifacts": [{"name", "revisions", "latest", "origin", "approved", "files"}]}`.

## Exit codes

The same on macOS, Linux and Windows.

| Code | Meaning | Examples |
|---|---|---|
| `0` | The command did what it promises | reached an Approval Gate or questions waiting for answers; run `succeeded`; Approval recorded; nothing to do |
| `1` | Unexpected error | a bug, I/O failure, unreadable database |
| `2` | Invalid usage | unknown flag, missing argument (clap's default); `answer` without a terminal and without `--accept`, `--file` or `-m` |
| `3` | **Refused**, nothing changed | precondition not met; Driver already live; ambiguous reference; Integration Branch moved; Git too old; pinned Revision edited; confirmation needed without `--yes`; `stop` without a Driver |
| `4` | **Blocked**: stopped needing a human outside an Approval Gate | Attempt or Iteration limit; `merge_into_base_interrupted` |
| `5` | The run was **cancelled** while this command drove it | `surt cancel` from another terminal |
| `130` | **Interrupted**, resumable | Ctrl-C, SIGHUP, console closed, `surt stop` |

- A `surt continue` that ends with `⚠ not merged: main moved since base` exits `0`. The delivery exists; the warning is in the text and in `status --json`.
- When some items are blocked and the rest merged, the command exits `4`.
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
  2 running executions will be stopped · 2/4 items merged stay on surt/20261017-add-login
  worktrees are removed; item branches and salvage refs stay until surt delete
  starting over later needs a new run (surt continue 20261017-add-login --new-run)
[y/N] y

$ surt restore fix-cache manifest
discard your unrecorded edit to the manifest of 20261016-fix-cache?
  tasks/B.md  +4 −1
  tasks/C.md  new file
  tasks/D.md  missing
[y/N] y
✔ restored manifest rev 1; your edit was discarded
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

# [cleanup]
# on_success = "keep"       # optional: a succeeded run keeps its worktrees and branches until surt delete
```

| Key | Default | When it applies | Decided in |
|---|---|---|---|
| `max_concurrent_agents` | `3` | read live by every Driver before each tick | [A](A-process-model.md#agent-concurrency-limit) |
| `max_attempts` | `3` | frozen at root run creation | [G](G-state-machines.md#limits) |
| `max_iterations` | `5` | frozen at root run creation | [G](G-state-machines.md#limits) |
| `[git] merge_into_base` | `"never"` | frozen; `--merge`/`--no-merge` on `surt run` and `surt continue --new-run` override it per run | [A. Git Integration](A-git-integration.md) |
| `[[checks]]` | none | frozen | [A. Git Integration](A-git-integration.md) |
| `[cleanup] on_success` | absent: the root Workflow Definition's (`"clean"`) | `"clean"` or `"keep"`; the effective policy is frozen | [I. Cleanup](I-cleanup.md#when-cleanup-runs), [E](E-workflow-schema.md#file-layout) |

The frozen keys and the per-run override form the `config` snapshot of [D](D-database.md#snapshots).

### Global: `~/.surt/config.toml` (optional)

On Windows the path is `%USERPROFILE%\.surt\config.toml`. `~/.surt/` also holds the worktrees ([A. Git Integration](A-git-integration.md)).

```toml
stop_grace_period = "10s"   # graceful stop before a forced kill
```

| Key | Default | When it applies | Decided in |
|---|---|---|---|
| `stop_grace_period` | `"10s"` | read by each process when it opens the project; never frozen into runs | [A. Process Model](A-process-model.md#supervising-agent-processes) |

- **What it governs.** `stop_grace_period` is how long every graceful stop of an agent process tree waits before a forced kill: Ctrl-C, a closed terminal, `surt stop` and `surt cancel` with a live Driver (the Claude Code runtime's stop), and Recovery stopping the surviving agents of a dead Driver. `surt stop`'s wait has no timeout of its own; it is bounded by this grace period.
- **Format.** The same duration format as a Check's `timeout`: a positive whole number with a unit `ms`, `s`, `m` or `h` (`"30s"`, `"2m"`). `"0s"`, `"ten"` and `"10"` are invalid.
- **Optional.** With no home directory or no file, the defaults apply.
- **Refusal.** The file is loaded with the project configuration by every command that opens the project (not by `init`). An unknown key, including a project key such as `max_concurrent_agents`, or a bad value refuses the command with exit `3`, naming the global file, the line and the reason:

```
✖ invalid configuration in /Users/me/.surt/config.toml:2
  unknown field `max_concurrent_agents`, expected `stop_grace_period`
  fix: edit /Users/me/.surt/config.toml:2
```

## Decided elsewhere

- Which commands drive, ownership refusals, Ctrl-C: [A. Process Model](A-process-model.md).
- Derived run states and the blocked lines: [G. State Machines](G-state-machines.md).
- Working Copy, edit detection, `--rev`, naming of Subjects: [H. Artifact Storage](H-artifact-storage.md).
- Recovery lines and refusals at start: [I. Recovery and Reconciliation](I-recovery.md).
- The stop and cancel request columns: [D. Database Schema](D-database.md#workflow-runs).
- What `delete` and cleanup remove, the cleanup lines of `status`, and the `--new-run` refusal for unrecorded edits: [I. Cleanup, Delete and Logs](I-cleanup.md).
- Which streams `logs` can read: [I. Cleanup](I-cleanup.md#logs).
