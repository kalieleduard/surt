# E. Workflow Definition Schema

How a Workflow Definition is written in TOML. Vocabulary (**Step**, **Outcome**, **Loop**, **Per-Item Step**, **Item List**, **Operation**) is defined in [`CONTEXT.md`](../../CONTEXT.md); entities and invariants are in [C. Domain Model](C-domain-model.md). Decision ticket: [Workflow TOML schema](../../.scratch/surt-architecture/issues/06-workflow-toml-schema.md).

## Not a language

The schema has a fixed set of step kinds and fields. It has no expressions, no templating (`{{ }}`), no JSONPath and no back-edges:

- Conditions compare one Outcome by equality, and nothing else.
- Repetition is a Loop step over a body of steps. The graph of `needs` stays acyclic.
- Data flows by artifact name. Steps never reference another step's output.

## File layout

```toml
schema = 1                    # format version, required
name = "task-cycle"           # Workflow Definition name, required
description = "Build, review and integrate one Task."   # optional

[cleanup]
on_success = "clean"          # "clean" (default) | "keep"

[inputs.request]              # root workflows only: what `surt run` must be given
format = "text"               # "text" | "markdown" | "json" | "git_ref"
optional = false              # default false
description = "what to build" # shown when an input is refused

[subject]
from = "request"              # optional: the text or markdown input the Subject is named after

[steps.review]                # one table per step; the id is unique by TOML itself
kind = "agent"                # required on every step
needs = ["check"]             # optional: steps that must finish first
when = { step = "check", outcome = "passed" }   # optional: run only on this Outcome
agent = "reviewer"            # the fields of the kind
```

`[cleanup] on_success` parses `"clean"` and `"keep"`, and the canonical form keeps the value. The **effective policy** of a root run is the project configuration's `[cleanup] on_success` when it is set ([J](J-cli.md#configuration)), else the root Workflow Definition's (`clean` for the Template's `feature`). Child definitions' values are not consulted: cleanup is per root run ([I](I-cleanup.md#when-cleanup-runs)). The effective policy is frozen into the `config` snapshot when the root run is created.

The `toml` crate is used with `preserve_order`. File order matters only for display, never for semantics. Unknown fields are errors (`deny_unknown_fields`).

### Run inputs: `[inputs]` and `[subject]`

A root workflow declares what a run needs in `[inputs.<name>]`. Each declared input is an **initial artifact** of the run: `surt run <workflow> --input <name>=<value>` ([J](J-cli.md#surt-run-create-a-run-and-drive-it)) supplies it, and the engine stores it as the first, human-origin Revision of the artifact of the same name, with one file:

| `format` | Value | File |
|---|---|---|
| `text` | non-empty UTF-8 text | `<name>.txt` |
| `markdown` | non-empty UTF-8 text | `<name>.md` |
| `json` | text that parses as JSON | `<name>.json`, the bytes given |
| `git_ref` | a branch, tag or commit, resolved when the run is created | `<name>.json` = `{"ref": <branch or null>, "sha": <full sha>}` |

- `optional` (default `false`): a required input must be given (or have a `default`). An optional one that is not given has no Revision.
- `default` is a string and is accepted only on a `git_ref` input (the Template's `base` defaults to `HEAD`); on any other format it is refused at `inputs.<name>.default`.
- `description` is free text shown in the input refusal.
- A `git_ref` that names a branch records the branch in `ref`; a tag or a commit records `null` (a detached base, [A](A-git-integration.md#the-base)).
- Only root workflows take inputs. A workflow used as a `per_item` child that declares `[inputs]` is refused at `inputs`: a child reads `item` and the parent's `pass`.
- The engine checks the values against the frozen root definition before it writes any row: an undeclared name, the same name twice, a required input missing, an empty `text`/`markdown`, `json` that does not parse, or a `git_ref` that names no commit refuses with `InvalidInput`, which lists every declared input with its format, whether it is optional and its description ([J](J-cli.md#surt-run-create-a-run-and-drive-it)).

`[subject] from = "<input>"` names the `text` or `markdown` input the Subject's slug is derived from; naming an undeclared input or another format is refused at `subject.from`. The slug comes from `--name` when given, else from that input's value, else from the workflow name. Both tables are part of the canonical form, hence of the definition's identity; a workflow that declares neither serializes as before.

### Where definitions live

A Workflow Definition is the file `.surt/workflows/<name>.toml` of the project; its file stem must equal its `name`, or the definition is refused at `name`. Nothing is compiled into the binary and nothing is merged over defaults ([ADR 0013](../adr/0013-definitions-live-in-the-project.md)): a project with no `.surt/workflows/` has no workflows. `surt init --template feature` writes the feature **Template** (`feature.toml`, `task-cycle.toml` and their five Agents, [below](#the-feature-template)) into `.surt/`, never overwriting a file that exists; from then on the files are the project's, versioned like any other code ([H](H-artifact-storage.md#what-is-versioned-in-the-users-git)).

`Project::create_run` reads the files when a root run is created, and reads only the closure it needs: the named workflow, every `per_item` child workflow transitively and every Agent those workflows reference (`.surt/agents/<id>/`, [F](F-agent-contract.md#definitions-in-the-project)). A broken workflow or Agent outside that closure does not refuse the run. No other command reads `.surt/workflows/`; they all use the frozen snapshots. A file that is not UTF-8 is refused; CRLF line endings are accepted, and the identity is computed over the canonical form, so CRLF and LF freeze alike.

## Step kinds

| `kind` | Who works | Fields | Outcomes |
|---|---|---|---|
| `agent` | an AI, through an Agent Runtime | `agent` (agent id), optional `inputs.<name> = "<artifact>"` renames | the `outcomes` its `agent.toml` declares; `done` if it declares none |
| `approval` | a human, via `surt approve` / `surt reject` | `artifact`, `rejection_artifact` | `approved` · `rejected` |
| `input` | a human, via `surt answer` | `artifact` (a `questions` artifact), `answer_artifact` | `answered` |
| `operation` | Surt's own code, no AI | `run` (operation id from Surt's registry) | declared by the operation in the registry |
| `loop` | the engine | `body` (step ids), `until = { step, outcome }` | `done` |
| `per_item` | the engine | `items` (an Item List artifact), `workflow`, `pass` (artifact names) | `done` |

Notes per kind:

- **agent:** the `agent.toml` owns the contract: `outcomes`, `[inputs]` (with `optional = true` where an input may not exist yet), `[outputs]` with the Outcomes that deliver each one (`outputs.<name>.outcomes`, [F](F-agent-contract.md#outputs-per-outcome)), any declared `item_files` on an Item List output, and whether the Agent keeps a conversation across its Loop (`[profile] conversation`, [F](F-agent-contract.md#a-conversation-across-a-loop)) ([section F](F-agent-contract.md)). The step does not repeat that list. It only renames, when a workflow's artifact name differs from the agent's input name.
- **approval:** it approves or rejects the **latest** Revision of `artifact` at the moment `surt approve`/`surt reject` runs, and records an Approval on that Revision. The `-m` reason of `surt reject` becomes a new human-origin Revision of `rejection_artifact`. Each gate needs its own `rejection_artifact` name, because names are per run and two gates sharing one name would hand one gate's reason to another agent.
- **input:** a human answers the questions of the **latest** Revision of `artifact`, which must be an artifact of format `questions` ([F](F-agent-contract.md#the-questions-format)). It waits like an approval gate: no Execution, no limit. Recording the answers publishes a new human-origin Revision of `answer_artifact` (format `json`, file `<answer_artifact>.json`) holding every earlier round plus this one ([H](H-artifact-storage.md#answers)), and the step ends `answered`. It runs only when the Agent asked, through `when`:
  ```toml
  [steps.answer-spec]
  kind = "input"
  artifact = "spec-questions"
  answer_artifact = "spec-answers"
  needs = ["spec"]
  when = { step = "spec", outcome = "questions" }
  ```
  `answer_artifact` shares one namespace with every `rejection_artifact`: no two steps may name the same one. The questions and answers are not Working Copy artifacts; only approval artifacts are. `surt approve` and `surt reject` act on approval steps only and refuse while only an input step waits.
- **operation:** this schema only references an Operation by id. The registry of ids, the Inputs each reads, their Outcomes and the artifacts they produce is in [A. Git Integration](A-git-integration.md#operation-registry). Operations take no arguments from the TOML.
- **loop:** see [Loops](#loops).
- **per_item:** see [Per-Item Steps](#per-item-steps).

A Step Run whose `when` is false ends as **SKIPPED**. SKIPPED is a state, not an Outcome, and it satisfies `needs` like any finished step.

## Artifacts and inputs

- A Workflow Run's Artifacts are identified by **name**. At the start of an Execution, each input resolves to the **latest Revision** of the artifact with that name, and that Revision is fixed as its Input.
- A required input with no Revision is a static validation error when no step before it (via `needs`) produces it. It is a runtime error otherwise. An optional input with no Revision is simply absent, for example `feedback` on the first Iteration.
- Steps read the latest Revision, not "the Revision a gate approved". The guarantee that nothing unapproved flows downstream is therefore enforced by the revalidation policy of [H. Artifact Storage, Freezing and Revalidation](H-artifact-storage.md): a driving command refuses while an artifact that passed a gate has a newer, unapproved Revision, and an approved Revision already read as an Input is pinned.
  ```
  $ surt continue 20261016-add-login
  ✖ spec changed after approval (rev 2 not approved)
    review it, then: surt approve 20261016-add-login
  ```
- Some names are supplied, not produced by a step:
  - **Initial artifacts** of a root run: its declared [run inputs](#run-inputs-inputs-and-subject), given to `surt run` and stored as human-origin Revisions. The feature Template declares two: `request` (`text`), what to build, and `base` (`git_ref`, default `HEAD`), the commit the run starts from ([A. Git Integration](A-git-integration.md#the-base)). The engine does not know what either means beyond its format.
  - `item`: in a Child Workflow Run, its item of the parent's Item List, together with that item's named files when present. It references the approved Item List Revision, not a new Revision.

## Ordering

- Order is explicit: `needs`. It is never inferred from inputs.
- Steps inside a Loop `body` may only `need` steps of the same body. Dependencies on steps outside the Loop are declared on the Loop step itself, and steps outside a Loop `need` the Loop, never a step of its body.
- The graph of `needs` (with each Loop treated as one node) must be acyclic.

## Loops

```toml
[steps.cycle]
kind = "loop"
body = ["build", "check", "review", "integrate"]
until = { step = "integrate", outcome = "merged" }
needs = ["setup"]
```

- Each Iteration creates **new Step Runs** for every step of the body. Finished Step Runs are never reopened.
- When every Step Run of the current Iteration is finished, the engine checks `until`. If the step ended with that Outcome, the Loop is `done`. Otherwise (another Outcome, or SKIPPED) a new Iteration starts.
- Artifacts produced in one Iteration are visible to the next one by name. That is how the Builder reads the Reviewer's `feedback`, and how the Spec Agent reads its previous `spec` plus `spec-rejection-reason`.
- The TOML declares **no iteration limit**. The limit is project configuration (`max_iterations`, default 5), applies only to Loops whose body has no `approval` or `input` step, and makes the Loop `blocked` when reached ([G. State Machines](G-state-machines.md#iterations)).

## Per-Item Steps

```toml
[steps.implement]
kind = "per_item"
items = "manifest"
workflow = "task-cycle"
pass = ["spec"]
needs = ["prepare"]
```

- `items` names an artifact in the **Item List** format. Its index is a JSON array where every element has a unique non-empty string `key`, a `needs` array of keys and an optional `files` map of file names to relative paths. Other fields are payload for the child:
  ```json
  [
    { "key": "A", "needs": [],    "title": "Create users table" },
    { "key": "B", "needs": ["A"], "title": "Login endpoint" },
    { "key": "C", "needs": [],    "title": "Login screen" }
  ]
  ```
- The producer's `agent.toml` declares any required item files and their formats through `outputs.<name>.item_files.<file-name>`. The `files` names must match those declarations on every item. Paths are rooted at the Execution's delivery directory and must resolve to safe regular files; nested Item Lists are not supported as item files. With no declarations, `files` may be absent or empty.
- An **Item List Revision** includes both its JSON index and all referenced file bytes. Approval covers that complete Revision; changing a referenced file creates a new Revision and requires reapproval. File paths alone are not a snapshot. The Template's Task Agent writes one Markdown file per Task through this generic mechanism; the engine does not interpret it as a Task document. Contract and examples: [F. Agent Contract](F-agent-contract.md#one-markdown-document-per-task); rationale: [ADR 0004](../adr/0004-item-list-files-share-approved-revision.md).
- An Item List is validated **when the producing agent delivers it** (duplicate or missing key, unknown key in `needs`, cycle, missing or invalid item file). An invalid list is an invalid output and leads to a new Attempt, so a human never approves a cyclic or incomplete list. The engine validates again on expansion, as a defence.
  ```
  ✖ plan-tasks attempt 1: invalid item list — cycle B → C → B
  ↻ plan-tasks attempt 2
  ```
- One Child Workflow Run of `workflow` starts per item, as soon as all the items it `needs` have finished. Ties follow list order. The Step Run finishes when every child is terminal. Items that need a blocked child wait for it; children are never cancelled one by one, so the step is `done` only when every child succeeded ([G. State Machines](G-state-machines.md#per-item-steps-and-blocked-children)).
- **Downward visibility:** a child reads its own artifacts, its `item` with that item's named files, and the parent artifacts listed in `pass`, read-only, by name. It does not receive other items' files. The Per-Item Step pins one Item List Revision on expansion; later Attempts do not reread mutable source files. Preparing local file bindings creates no new Revision. Static validation checks that every required input of the child workflow's agents is produced inside the child, is `item`, or is in the parent's `pass`.
- **No upward visibility:** the parent does not read children's artifacts. Their work reaches the parent through the Integration Branch, which the `snapshot` Operation reads from Git ([A. Git Integration](A-git-integration.md#snapshot-shipper-and-delivery)).
- Item keys must match `^[A-Za-z0-9][A-Za-z0-9_-]{0,31}$`, be unique case-insensitively and not be a Windows reserved device name; item file path components follow matching rules. These checks are part of Item List validation, so they apply to every workflow ([A. Git Integration](A-git-integration.md#item-keys-and-item-file-paths)).

## Freezing and identity

- A root Workflow Run freezes, at creation, the **closure** of its effective definitions: its own and every Workflow Definition reachable through `per_item` steps. Its Child Workflow Runs use the frozen copy. Edits never reach a run that already exists.
- A frozen definition is identified by the SHA-256 of its canonical form, shown short in `status`/`inspect` (`feature@3f9a2c · task-cycle@8b1d07`). There is no hand-maintained `version` field.
- `schema` is the format version. A binary refuses a definition whose `schema` it does not know.
- Agent contracts, instructions, schemas and each Agent's profile are also frozen at root run creation, as read from `.surt/agents/`; see [F. Agent Contract](F-agent-contract.md).

## Static validation

Validation runs on every definition of the closure before a root Workflow Run is created, and the run is not created if it fails: the refusal is `InvalidDefinition` (exit `3`, no row written), carrying the file under `.surt/`, the field path or the step when one applies, and the reason. The feature Template is also validated in Surt's own test suite, through the engine, as a project would. A missing root workflow is `UnknownWorkflow`, which lists the workflows found in `.surt/workflows/`, or suggests `surt init --template feature` when there are none:

```
✖ .surt/workflows/task-cycle.toml:12: unknown field `agnet`, expected one of `kind`, `agent`, …
  fix: edit .surt/workflows/task-cycle.toml
✖ .surt/workflows/task-cycle.toml: steps.cycle: expects outcome "merge" of "integrate", which declares: merged, conflict
  fix: edit .surt/workflows/task-cycle.toml
✖ no workflow is named "feature" in .surt/workflows/
  .surt/workflows/ holds no workflow
  fix: surt init --template feature
```

Checks:

1. Known `schema`; required top-level fields; no unknown fields anywhere.
2. Every step has a known `kind` and exactly the fields of that kind.
3. Every id in `needs`, `body`, `when.step` and `until.step` exists. `body` members belong to exactly one Loop, and Loops do not nest.
4. The `needs` rules of [Ordering](#ordering) hold, and the graph is acyclic.
5. `when` and `until` reference an Outcome the referenced step's kind, agent or operation declares. `until.step` is in the Loop's `body`.
6. Every `agent` exists. Every required input of an agent or Operation is produced by a step before it (via `needs`, including earlier steps of the same body), is a declared input of the root workflow (`[inputs]`), is `item`, or is passed by the parent (`pass`). An artifact a Git Operation reads as a Git reference (`base`, see [the Operations' artifact contract](#the-operations-artifact-contract)) must come from a `git_ref` input; any other source is refused at the step.
7. Every `run` is a registered operation. Every `workflow` of a `per_item` exists, and its `items` artifact is produced by a step before it in the Item List format. Any `item_files` declarations have valid names and supported file formats; actual item references and bytes are validated at delivery and expansion.
8. Every `approval` names an `artifact` produced before it and a `rejection_artifact` unique in the definition. Every `input` names an `artifact` produced before it in the `questions` format and an `answer_artifact`; `rejection_artifact` and `answer_artifact` names are unique together (`DuplicateRejectionArtifact`, `DuplicateAnswerArtifact`, `NotQuestions`). An `input` step produces its `answer_artifact` as `json` for the steps after it.
9. `[inputs]`: known formats and fields, `default` only on `git_ref`, none in a `per_item` child; `[subject] from` names a declared `text` or `markdown` input.
10. A step whose Agent has `conversation = "loop"` is in a Loop's `body` (`ConversationOutsideLoop`: `agent "<agent>" has conversation = "loop", but the step is in no loop's body`).

## The Operations' artifact contract

Operations read and write artifacts by fixed names; these names are the contract a workflow must satisfy around each Operation (Outcomes and Git behaviour: [A. Git Integration](A-git-integration.md#operation-registry)).

| Operation | Reads | Writes | Outcomes |
|---|---|---|---|
| `git.create_run_branch` | `base` (a `git_ref` input) | — | `done` |
| `git.create_child_worktree` | `item` | — | `done` |
| `checks.run` | `commit` | `check-report` | `passed` · `failed` |
| `git.integrate` | `commit`, `feedback` | `conflict` (on conflict) | `merged` · `conflict` |
| `git.snapshot_run_branch` | `base` (a `git_ref` input) | `integration` | `done` |
| `git.merge_into_base` | `base` (a `git_ref` input), `integration` | — | `merged` · `skipped` |

`base` is read as a Git reference, so a root workflow that runs any of the three Operations reading it declares `[inputs.base] format = "git_ref"`. The JSON artifacts the engine writes (`check-report`, `conflict`, `integration`) and `base` itself follow the schemas the engine embeds; an Agent that reads one references its own copy in `schemas/` ([F](F-agent-contract.md)).

## The feature Template

These are the files `surt init --template feature` writes to `.surt/workflows/`. Agent ids are `spec`, `tasks`, `builder`, `reviewer` and `shipper` (`.surt/agents/<id>/`, layout in section F). The comments listing what each agent reads and writes are informative. The contract lives in each `agent.toml`, and `?` marks an optional input.

### `feature.toml`

```toml
schema = 1
name = "feature"
description = "Specify, plan, build and ship one Feature."

[cleanup]
on_success = "clean"

[inputs.request]
format = "text"
description = "what to build"

[inputs.base]
format = "git_ref"
default = "HEAD"
description = "branch, tag or commit to start from"

[subject]
from = "request"

# --- spec: the Spec Agent asks or writes it, you answer, approve or reject --

[steps.spec]
kind = "agent"
agent = "spec"            # reads: request, spec?, spec-rejection-reason?, spec-answers?  writes: spec (done) or spec-questions (questions)

[steps.answer-spec]
kind = "input"
artifact = "spec-questions"
answer_artifact = "spec-answers"
needs = ["spec"]
when = { step = "spec", outcome = "questions" }

[steps.approve-spec]
kind = "approval"
artifact = "spec"
rejection_artifact = "spec-rejection-reason"
needs = ["answer-spec"]
when = { step = "spec", outcome = "done" }

[steps.spec-loop]
kind = "loop"
body = ["spec", "answer-spec", "approve-spec"]
until = { step = "approve-spec", outcome = "approved" }

# --- tasks: the Task Agent breaks the spec into an Item List --------------

[steps.plan-tasks]
kind = "agent"
agent = "tasks"           # reads: spec, manifest?, manifest-rejection-reason?  writes: manifest + its per-item Markdown files

[steps.approve-tasks]
kind = "approval"
artifact = "manifest"
rejection_artifact = "manifest-rejection-reason"
needs = ["plan-tasks"]

[steps.tasks-loop]
kind = "loop"
body = ["plan-tasks", "approve-tasks"]
until = { step = "approve-tasks", outcome = "approved" }
needs = ["spec-loop"]

# --- build: one task-cycle per Task, then ship ----------------------------

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"   # reads: base
needs = ["tasks-loop"]

[steps.implement]
kind = "per_item"
items = "manifest"
workflow = "task-cycle"
pass = ["spec"]
needs = ["prepare"]

[steps.snapshot]
kind = "operation"
run = "git.snapshot_run_branch"   # reads: base  writes: integration
needs = ["implement"]

[steps.ship]
kind = "agent"
agent = "shipper"         # reads: manifest, integration  writes: delivery
needs = ["snapshot"]

[steps.merge]
kind = "operation"
run = "git.merge_into_base"        # fast-forwards the Base branch only if [git] merge_into_base = "ff-only"
needs = ["ship"]
```

### `task-cycle.toml`

```toml
schema = 1
name = "task-cycle"
description = "Build, review and integrate one Task."

[cleanup]
on_success = "clean"

[steps.setup]
kind = "operation"
run = "git.create_child_worktree"

# --- one Iteration: build, check, review, integrate -----------------------

[steps.build]
kind = "agent"
agent = "builder"         # reads: item + its task Markdown file, spec, feedback?, check-report?, conflict?  writes: commit

[steps.check]
kind = "operation"
run = "checks.run"        # reads: commit  writes: check-report
needs = ["build"]

[steps.review]
kind = "agent"
agent = "reviewer"        # reads: item + its task Markdown file, spec, commit, check-report, feedback?, conflict?  writes: feedback
needs = ["check"]
when = { step = "check", outcome = "passed" }

[steps.integrate]
kind = "operation"
run = "git.integrate"     # reads: commit, feedback  writes: conflict (on conflict)
needs = ["review"]
when = { step = "review", outcome = "approved" }

[steps.cycle]
kind = "loop"
body = ["build", "check", "review", "integrate"]
until = { step = "integrate", outcome = "merged" }
needs = ["setup"]
```

### Steps and Outcomes

| Workflow | Step | Kind | Performed by | Outcomes |
|---|---|---|---|---|
| feature | `spec-loop` | loop | engine | `done` when `approve-spec` = `approved` |
| feature | `spec` | agent | Spec Agent (`spec`) | `done` · `questions` |
| feature | `answer-spec` | input | human | `answered` |
| feature | `approve-spec` | approval | human | `approved` · `rejected` |
| feature | `tasks-loop` | loop | engine | `done` when `approve-tasks` = `approved` |
| feature | `plan-tasks` | agent | Task Agent (`tasks`) | `done` |
| feature | `approve-tasks` | approval | human | `approved` · `rejected` |
| feature | `prepare` | operation | Surt (`git.create_run_branch`) | `done` |
| feature | `implement` | per_item | engine, one `task-cycle` per Task | `done` |
| feature | `snapshot` | operation | Surt (`git.snapshot_run_branch`) | `done` |
| feature | `ship` | agent | Shipper (`shipper`) | `done` |
| feature | `merge` | operation | Surt (`git.merge_into_base`) | `merged` · `skipped` |
| task-cycle | `setup` | operation | Surt (`git.create_child_worktree`) | `done` |
| task-cycle | `cycle` | loop | engine | `done` when `integrate` = `merged` |
| task-cycle | `build` | agent | Builder (`builder`) | `done` |
| task-cycle | `check` | operation | Surt (`checks.run`) | `passed` · `failed` |
| task-cycle | `review` | agent | Reviewer (`reviewer`) | `approved` · `changes_requested` |
| task-cycle | `integrate` | operation | Surt (`git.integrate`) | `merged` · `conflict` |

How the spec Loop behaves:

- `spec` = `questions` → `answer-spec` waits for `surt answer`; once it is `answered`, `approve-spec` is SKIPPED → a new Iteration starts, and the Spec Agent reads `spec-answers`.
- `spec` = `done` → `answer-spec` is SKIPPED and `approve-spec` waits. `rejected` → a new Iteration starts, and the Spec Agent reads `spec-rejection-reason`; `approved` → the Loop is `done`.
- The Spec Agent has `conversation = "loop"`, so each Iteration continues the conversation of its last successful Execution and is sent only the Inputs that changed ([F](F-agent-contract.md#a-conversation-across-a-loop)). The Loop has a human step in its body, so it has no Iteration limit.

How the Task Cycle behaves:

- `check` = `failed` → `review` and `integrate` are SKIPPED → a new Iteration starts, and the Builder reads `check-report`.
- `review` = `changes_requested` → `integrate` is SKIPPED → a new Iteration starts, and the Builder reads `feedback`.
- `integrate` = `conflict` → a new Iteration starts. The Builder reads `conflict`, merges the Integration Branch tip into its Item Branch, and the new commit goes through `check` and `review` again ([A. Git Integration](A-git-integration.md#conflicts)). The review covers the whole commit, focused on the resolution delta ([G](G-state-machines.md#what-the-reviewer-reads-after-the-first-iteration)).
- `review` = `approved` and `integrate` = `merged` → the Loop is `done` and the Task is complete.

Operations, Checks, the `check-report` and `conflict` artifacts, and the opt-in merge into the Base are specified in [A. Git Integration](A-git-integration.md).

## Decided elsewhere

- Agent contract format (`outcomes`, `[inputs]`, `[outputs]`, `[profile]`, freezing of agents): section F ([Agent contract](../../.scratch/surt-architecture/issues/07-agent-contract.md)).
- Iteration and Attempt limits, Task blocking, and what a blocked child means for its dependents: [G. State Machines](G-state-machines.md).
- Revalidation after an approved artifact is edited: [H. Artifact Storage, Freezing and Revalidation](H-artifact-storage.md).
- Operation registry, Integration Branch, Checks, item key constraints: [A. Git Integration](A-git-integration.md) ([Git integration model](../../.scratch/surt-architecture/issues/09-git-integration-model.md)).
- What `on_success = "clean"` removes, what `"keep"` keeps, and what a cancelled run removes regardless of either: [I. Cleanup, Delete and Logs](I-cleanup.md#what-each-ending-removes).
- `surt reject`, and which command drives after it (a fixed table in the CLI, not a schema field): [J. CLI Surface](J-cli.md#next-command-hints).
