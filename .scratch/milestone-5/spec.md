# Spec: Milestone 5 — the project owns its workflows

Labels: ready-for-agent
Type: spec
Status: open (6 of 7 tickets resolved; 07 awaits CI and the manual smoke; tickets in [issues/](issues/))
Map: [Surt — Architecture and implementation plan](../surt-architecture/map.md)
Plan: [K. Roadmap](../../docs/architecture/K-roadmap.md#after-milestone-1), [E. Workflow Definition Schema](../../docs/architecture/E-workflow-schema.md), [F. Agent Contract](../../docs/architecture/F-agent-contract.md), [J. CLI](../../docs/architecture/J-cli.md), [ADR 0013](../../docs/adr/0013-definitions-live-in-the-project.md)
Previous: [Milestone 4](../milestone-4/spec.md)

## Problem Statement

Surt's engine is generic, but the `surt` binary is not. The feature workflow (spec → tasks → build/review per Task → ship → merge) is compiled into it, and the CLI is that workflow's front end:

- **The definitions are not the project's.** `feature.toml`, `task-cycle.toml` and the five Agents live in `crates/surt/assets/` and reach the engine as a `Catalog` built with `include_str!`. M4 lets a project override fields of an Agent under `.surt/agents/<id>/`, but a developer cannot read the effective definition without opening the binary's sources, cannot change a workflow at all, and cannot add one. The map put "project-specific workflows" out of scope; using Surt on real repositories shows that the workflow, not only the Agents, is what a team wants to shape.
- **The CLI knows the feature workflow.** `surt feature`, `surt plan` and `surt start` are hard-coded phases over the gates `approve-spec` and `approve-tasks` (`phase.rs`). `surt feature "<description>"` seeds the artifacts `request` and `base`, as if every workflow had a description and a Git base. Delivery checks are wired to the agent ids `builder` and `reviewer` in Rust. Status, progress and refusals say "feature", "task" and "shipper".
- **Some feature vocabulary leaked into the engine.** The Working Copy lives under `.surt/features/`, child branches are `surt/task/…`, an integration merge says "Merge task", the integration artifact lists `tasks`, and `StatusScope::Feature` names the Subject.

## Solution

Milestone 5 ("the project owns its workflows") moves every Workflow Definition and Agent into the project as plain files, and makes the CLI generic over them. The feature workflow becomes a **Template** the binary can write once into `.surt/`.

```
$ surt init --template feature
✔ initialized .surt/
  .surt/.gitignore                         created
  .surt/config.toml                        created
  .surt/surt.db                            created
  .surt/workflows/feature.toml             created
  .surt/workflows/task-cycle.toml          created
  .surt/agents/builder/agent.toml          created
  …
  next: surt run feature --input request="<what you want to build>"

$ surt run feature --input request="add login" --input base=main
✔ created 20261019-add-login   workflow: feature   base: main @ a11c9d0
…
⏸ spec rev 1 awaiting approval
  approve: surt approve add-login
  reject:  surt reject add-login -m "<reason>"

$ surt approve add-login
✔ spec rev 1 approved
  next: surt continue add-login

$ surt continue add-login
```

- **Definitions live in `.surt/`.** `.surt/workflows/<name>.toml` and `.surt/agents/<id>/{agent.toml,instructions.md,schemas/…}` are the complete definitions. The engine reads them when a root run is created and freezes them exactly as it froze the built-ins. There is no compiled `Catalog`, no merge over defaults and no fallback: a project with no `.surt/workflows/` has no workflows.
- **The feature workflow is a Template.** `surt init --template feature` writes the files the binary used to embed, never overwriting a file that exists. Once written they are the project's: edit, rename, delete or add any of them.
- **A workflow declares its run inputs.** `[inputs.<name>]` in the workflow says what a run needs: a value in a given `format` (`text`, `markdown`, `json`) or a Git reference (`format = "git_ref"`, resolved to `{ref, sha}`). `surt run <workflow> --input name=value` supplies them; a workflow with no inputs runs with `surt run <workflow>`. `[subject] from = "<input>"` names the input the Subject's slug is derived from.
- **Delivery checks are declared on outputs.** `[outputs.commit] checks = [{ kind = "commit_evidence" }]` replaces `Catalog::delivery_check`, and the checks are frozen with the Agent.
- **The CLI is generic.** `surt run` creates a run and drives it to the next gate; `surt continue <subject>` drives it again (`--new-run` starts the next run of the Subject). `feature`, `plan` and `start` are removed. Every other command takes a `<subject>`; the wording says subject and item, labels gates by their artifact and items by their `per_item` step.
- **The engine stops naming the feature workflow.** The Working Copy moves to `.surt/subjects/<subject>/`, child branches become `surt/item/…`, integration merges say "Merge item", the integration artifact lists `items`, `StatusScope::Feature` becomes `StatusScope::Subject`.

v1 is not released, so nothing here migrates old projects: a project initialized before M5 runs `surt init --template feature` once. The database schema does not change.

## User Stories

### Definitions in the project

1. As a developer, I want `surt init --template feature` to write the feature workflow and its five Agents into `.surt/`, so that I can read exactly what Surt will run.
2. As a developer, I want to edit `.surt/agents/reviewer/instructions.md` and have the next run use it, so that the Reviewer follows my project's conventions.
3. As a developer, I want to edit `.surt/workflows/feature.toml` (remove the `merge` step, add an approval, change a loop), so that the flow fits my team.
4. As a developer, I want to add `.surt/workflows/<name>.toml` with its own Agents, so that Surt runs a workflow that is not about building features.
5. As a developer, I want `surt init --template feature` on an initialized project to add only the missing files and say which ones it kept, so that I can restore a deleted file without losing my edits.
6. As a developer, I want plain `surt init` to write no workflow, so that a project that defines its own workflows starts empty.
7. As a developer, I want `.surt/workflows/` and `.surt/agents/` versioned by the generated `.gitignore`, so that the team shares them.
8. As a developer, I want a run already created to keep the definitions it froze, whatever happens to the files, so that editing a workflow never changes a run in progress.
9. As a developer, I want an invalid workflow or Agent file refused with its path, the field and the reason, so that I can fix it.
10. As a developer, I want a broken Agent that no workflow I run uses not to block my runs, so that work in progress in one file does not stop the others.
11. As a developer, I want an unknown `--template` refused with the list of templates, so that I know what exists.

### Run inputs

12. As a developer, I want a workflow to declare the inputs a run needs, so that a workflow without a description runs without one.
13. As a developer, I want `--input name=value` and `--input name=@path`, so that a long request can come from a file.
14. As a developer, I want a missing required input, an unknown input or a value that does not match its format refused with the declared inputs listed, so that I see what to pass.
15. As a developer, I want a `git_ref` input resolved to a commit when the run is created, with an optional `default`, so that `base` defaults to `HEAD` as before.
16. As a developer, I want the Subject's name derived from the input `[subject] from` names, or from `--name`, or else from the workflow name, so that names stay readable.
17. As a developer, I want a workflow whose steps consume an artifact no input or step produces refused before any row is written, so that a missing `base` is caught at creation.

### Delivery checks

18. As a developer, I want `commit_evidence`, `reviewed_commit` and `no_blocking_finding` declared on an Agent's output, so that a renamed Builder or Reviewer keeps its guarantees.
19. As a developer, I want the checks frozen with the Agent, so that a binary upgrade never changes the checks of a run in progress.
20. As a developer, I want a check on an output that is not JSON, or `no_blocking_finding` with an Outcome the Agent does not declare, refused at creation.

### The CLI

21. As a developer, I want `surt run <workflow>` to create the run and drive it until a gate or the end, so that one command starts any workflow.
22. As a developer, I want `surt run` to refuse when a Subject with the same slug has an active run, pointing at `surt continue`, unless I pass `--new`.
23. As a developer, I want `surt continue <subject>` to drive the active run again, so that resuming does not depend on which phase the workflow is in.
24. As a developer, I want `surt continue <subject> --new-run` to start the next run of the same workflow with the same inputs (Git references resolved again), and `--input` to change one, so that a retry needs no retyping.
25. As a developer, I want `--merge`/`--no-merge` on `surt run` and `surt continue --new-run`, refused on a plain `continue`, as M4 had them on `surt feature`.
26. As a developer, I want `approve`, `reject`, `status` and `stop` to print `surt continue <subject>` as the next command.
27. As a developer, I want status, progress, `show`, `inspect`, `logs` and refusals to say subject and item, and to label a gate by its artifact and an item list by its `per_item` step, so that the output fits any workflow.

## Implementation Decisions

### Definitions on disk

- **Layout.** `.surt/workflows/<name>.toml` (the file stem must equal `name`); `.surt/agents/<id>/agent.toml` (required, `id` must equal the directory name), `instructions.md` (required), `schemas/**` (optional). `.gitkeep`/`.keep` placeholders are ignored as in M4.
- **Where they are read.** `Project::create_run` (root runs only) reads the closure it needs: the named workflow, every `per_item` child workflow transitively, and every Agent those workflows reference. Files outside the closure are not read, so a broken unrelated Agent or workflow does not refuse (story 10). Nothing else reads `.surt/workflows/` or `.surt/agents/`; every other command keeps reading the frozen snapshots.
- **Validation** is the union of what M1–M4 applied to built-ins and to overrides, now applied to every file: the workflow schema and static validation (E), the complete `agent.toml` schema with `deny_unknown_fields`, profile checks, schema paths present, local `$ref`s only, schemas compile, `runtime = "fake"` only with `test-support`, non-UTF-8 refused, CRLF accepted with the snapshot identity computed over canonical TOML.
- **Refusal.** `InvalidDefinition` carries the file path under `.surt/`, the field path or step when relevant, and the reason; J's message shape, exit `3`; no row is written. `UnknownWorkflow` lists the workflows found in `.surt/workflows/` and, when there are none, says `surt init --template feature`. The "invalid built-in definition … reinstall surt" wording goes away.
- **Engine operation schemas.** `check-report`, `conflict`, `integration` and `base` stay owned by the engine, which validates its own Operations' outputs against its embedded copies. An Agent that reads one of them as an input references a file in its own `schemas/` like any other schema; the Template writes those copies from the engine's exported constants.
- **What goes away.** The `Catalog` argument of `Project::open`, `AgentAssets`, the M4 merge (`with_local`, effective TOMLs, field-by-field merge, `blame_overrides`), the "not a built-in Agent id" hygiene refusal and the `local agents:` creation line. `Catalog` may survive as the engine's internal in-memory form of what was read, with owned strings.

### Template

- **Owner.** The binary owns the Template's bytes (`crates/surt/assets/templates/feature/…`, embedded with `include_str!` as today); the engine owns writing files into `.surt/`. `Project::init` takes the Template's files as data (`&[(rel_path, bytes)]`), writes the missing ones, keeps the existing ones and reports each file's state. The engine does not know any Template name.
- **CLI.** `surt init [--template <name>]`. Known templates: `feature`. An unknown name is a refusal listing them (exit `3`). Rerunning init never overwrites any file; the report says `created` or `kept` per file.
- **`.gitignore`** versions `config.toml`, `workflows/**` and `agents/**`.
- **Content.** The Template is today's `feature.toml`, `task-cycle.toml` and five Agents, plus the `[inputs]`/`[subject]` tables and the output `checks` this milestone adds. The `next:` line of init prints `surt run feature --input request="<what you want to build>"` when the Template was written, and `add a workflow under .surt/workflows/` otherwise.
- **Template tests.** The current `builtin/tests.rs` assertions (the closure validates, `merge` is last, five Agents, profiles, tools, checks) move to the Template, validated through the engine as a project would.

### Run inputs

- **Schema.** A workflow may declare `[inputs.<name>]` with `format = "text" | "markdown" | "json" | "git_ref"`, `optional` (default `false`), `default` (a string; only for `git_ref`), `description` (shown in refusals). Only root workflows use inputs; a `per_item` child declaring `[inputs]` is refused (its artifacts come from `item` and `pass`). Unknown fields are refused.
- **Artifacts.** Each supplied input becomes an initial artifact of the same name with one file: `<name>.txt`, `<name>.md`, `<name>.json`, or for `git_ref` `<name>.json` holding `{"ref": <branch or null>, "sha": <full sha>}` (today's `base.json`). `json` values must parse; `text`/`markdown` must be non-empty UTF-8.
- **Static validation.** Declared inputs are the initial artifact names given to `flow::check`, replacing the names the CLI passed (`request`, `base`). An Operation that needs `base` therefore requires an input named `base` with `format = "git_ref"`; any other format for an artifact consumed by a Git Operation is refused.
- **`[subject] from = "<input>"`** names a `text` or `markdown` input. Slug source order: `--name`, else that input's value, else the workflow name. `Slug::derive`, the date prefix and `-N` suffixes are unchanged.
- **CLI.** `--input <name>=<value>` repeatable; `<name>=@<path>` reads the file (relative to the current directory). An unknown name, a duplicate, a missing required input, a `git_ref` that does not name a commit, or a value of the wrong format is a refusal listing every declared input with its format, `optional` and `description`.
- **Creation line.** `created <subject>   workflow: <name>` plus `   <input>: <ref> @ <short sha>` for each `git_ref` input, and `merge into base: …` as in M4.

### Delivery checks

- **Schema.** An output may declare `checks = [{ kind = "commit_evidence" }, { kind = "reviewed_commit" }, { kind = "no_blocking_finding", outcome = "<outcome>" }]`. The output must be `format = "json"` and `no_blocking_finding.outcome` must be one of the Agent's outcomes; the existing static checks move from `closure.rs`'s `check_deliveries` into `AgentContract::check`.
- **Frozen.** Checks are part of the canonical `agent.toml`, hence of the Agent snapshot identity. `launch.rs` reads them from the frozen Agent instead of the live catalog. The implementations in `drive/evidence.rs` and the input/field names they read are unchanged.

### The CLI

- **`surt run <workflow> [--input k=v]… [--name <slug>] [--new] [--merge|--no-merge]`** opens the project, refuses an active Subject with the same slug unless `--new` (pointing at `surt continue <subject>`), creates the root run and drives it.
- **`surt continue <subject>`** finishes a pending cleanup (as the phases did), then drives the active run; with no active run it refuses and suggests `--new-run`. `--merge`/`--no-merge` without `--new-run` refuse when the merge choice is already frozen, as M4's resume form did.
- **`surt continue <subject> --new-run [--input k=v]… [--merge|--no-merge]`** creates the next run of the workflow the Subject's latest run used, read from `.surt/` now. Non-`git_ref` inputs are carried from the previous run (the existing `carried` mechanism) unless `--input` replaces them; `git_ref` inputs are resolved again from `--input` or their `default`. An input the workflow now requires and the previous run did not have must be passed.
- **Removed:** `surt feature`, `surt plan`, `surt start`, `phase.rs`, `builtin.rs` as a catalog, the gate entry checks ("approve the spec first"). A `continue` while a gate waits simply ends at that gate again.
- **Next commands.** `approve`/`reject`, `status`, `stop`, interrupted and blocked runs print `surt continue <subject>`; reject says the producing step will run again instead of naming "the Spec Agent".
- **Wording and JSON.** Positional arguments are `<subject>`; `--task` becomes `--item` on `show`, `inspect` and `logs`. Status headers say `SUBJECT`; gate rows are labelled by the gate's artifact and list item files when the artifact is an Item List with files (count from the Revision, not from a hard-coded index); the items section is titled by the `per_item` step key; progress says `item <key>`; the "delivered" line names the step and prints only when an integration branch exists. JSON keys: `features` → `subjects`, `feature` → `subject`, `tasks` → `items`, `task` → `item`. `.surt/features/…` paths in messages become `.surt/subjects/…`.

### Engine vocabulary

- **Working Copy:** `.surt/subjects/<subject>/` (layout, cleanup, delete, restore, refusals).
- **Child branches:** `surt/item/<stem>/<key>`. **Integration merge:** message `Merge item <key> …` with trailer `Surt-Item:`.
- **Integration artifact:** key `items` in `integration.schema.json` and `drive/snapshot.rs`; the Template's Shipper instructions follow.
- **`StatusScope::Feature` → `StatusScope::Subject`**; `base.schema.json`'s title no longer says "surt feature".
- Operation artifact names (`base`, `item`, `commit`, `feedback`, `check-report`, `conflict`, `integration`) stay: they are the Operations' contract, documented in E.

### Documentation updates

- **ADR 0013** (new): definitions live in the project; the binary only writes Templates. Amends ADR 0011's "built-in definitions reach the engine as a `Catalog`" and F's override model; row in L.
- **CONTEXT.md:** Workflow Definition and Agent are files under `.surt/`; new terms **Template** and **Run Input**; **Agent Override** removed; the "Feature workflow (built-in)" section becomes the feature Template.
- **E:** the files, `[inputs]`, `[subject]`, the Operations' artifact contract, no built-in definitions. **F:** the files are complete definitions, output `checks`, no merge. **J:** `init --template`, `run`, `continue`, `--input`, `--item`, JSON keys, next commands. **A. Process Model:** the phase table becomes run/continue. **B:** no `Catalog` in `Project::open`, the Template in `surt`. **H:** `.surt/subjects/`, versioned `workflows/`. **A. Git Integration:** branch and trailer names. **K:** the M5 row; the map's "project-specific workflows" line lifted.
- **Smoke checklist:** an M5 section.

## Testing Decisions

- **Seams** stay those of M1–M4: `Project` with the Fake Runtime and real Git for the engine, the binary with TOML Fake scripts for the CLI. Tests check rows, snapshots, refs, exit codes and output, never private loading state.
- **Engine fixtures** write their synthetic workflows and Agents into the test project's `.surt/` instead of passing a `Catalog`; a helper keeps today's inline TOML constants.
- **Loading:** a closure is read from disk and frozen; editing a file after creation changes nothing for that run; an unrelated broken Agent does not refuse; a broken referenced one refuses with path and field and writes no row; workflow file stem ≠ `name` refuses; CRLF and LF freeze identically.
- **Inputs:** each format becomes the right artifact; `git_ref` default and explicit; missing, unknown, duplicate, malformed and `@file` inputs; `[subject] from`; a `base`-consuming Operation without a `git_ref` `base` input refuses.
- **Checks:** declared checks are frozen and enforced; editing `agent.toml` mid-run does not change them; invalid declarations refuse.
- **CLI:** `init` with and without `--template`, rerun keeps files, unknown template; `run`, `continue`, `--new-run` with carried and replaced inputs, `--merge` rules; next-command lines; the generic wording and JSON keys. The existing CLI suites move from `feature`/`plan`/`start` to `run`/`continue` with `sandbox.init()` writing the Template.
- **CI** as before: fmt, clippy `-D warnings`, `cargo test --features test-support` on macOS, Linux and Windows.

## Out of Scope

- Updating already-materialized Template files on upgrade, a diff against the Template, or a `surt template` command.
- More Templates than `feature`; Templates outside the binary (URLs, directories).
- New Operations, new delivery check kinds, or workflow-declared CLI phases.
- Migrating Working Copies or branches of runs created before M5.
- Nested loops or other schema features E rules out.

## Further Notes

- **Trust.** `.surt/workflows/` and `.surt/agents/` are versioned code: pulling a branch can change what a new run does and which tools its Agents may run. They are reviewed like any other code (F's note on overrides now covers whole definitions).
- **Upgrades.** A new Surt release no longer changes what an existing project runs; it can only change what `surt init --template feature` writes into a fresh project or for a deleted file. That is the price of owning the files, recorded in ADR 0013.
