# 03: A workflow declares its run inputs; `surt run` supplies them

Spec: [Milestone 5](../spec.md) (stories 12–17, 21, 22, 25) · Plan: [E](../../../docs/architecture/E-workflow-schema.md), [J](../../../docs/architecture/J-cli.md)

**What to build:** `[inputs.<name>]` and `[subject]` in a root workflow say what a run needs and where its name comes from. `surt run <workflow> --input name=value` creates the run and drives it to the next gate, replacing the creation form of `surt feature "<description>"`.

```
$ surt run feature --input request="add login" --input base=main --merge
✔ created 20261019-add-login   workflow: feature   base: main @ a11c9d0
  merge into base: ff-only (--merge)
```

**Blocked by:** 02

**Status:** resolved

- [x] Workflow schema: `[inputs.<name>]` with `format = "text" | "markdown" | "json" | "git_ref"`, `optional`, `default` (`git_ref` only), `description`; `[subject] from = "<text or markdown input>"`; a `per_item` child declaring inputs is refused; both are in the canonical form
- [x] Declared inputs are the initial artifact names of static validation; an artifact consumed by a Git Operation must come from a `git_ref` input, otherwise refused
- [x] `NewRun` carries input values; the engine builds the artifacts (`<name>.txt|.md|.json`, `git_ref` → `<name>.json` `{ref, sha}`), validating format, required, unknown and duplicate inputs and resolving `git_ref` (with `default`) before any row is written
- [x] Slug: `--name`, else `[subject] from`'s value, else the workflow name
- [x] `surt run <workflow> [--input k=v | k=@path]… [--name] [--new] [--merge|--no-merge]`; an active Subject with the same slug refuses pointing at `surt continue <subject>` unless `--new`; input refusals list every declared input with format, optional and description
- [x] The creation line prints `workflow:` and each `git_ref` input; `surt feature "<description>"` (creation) is removed; resume forms of `feature`, `plan` and `start` stay until 04
- [x] The feature Template declares `request` (`text`) and `base` (`git_ref`, default `HEAD`) and `[subject] from = "request"`; init's `next:` line prints `surt run feature --input request="<what you want to build>"`
- [x] Tests: each format, `@file`, defaults, every refusal, `[subject] from`, a workflow with no inputs, a `base`-consuming Operation without a `git_ref` input; CLI creation tests move to `surt run`
- [x] E documents `[inputs]`, `[subject]` and the Operations' artifact contract; J documents `surt run`

## Comments

- Resolved on `m5/03-run-inputs`. `NewRun { subject, workflow, inputs: Vec<(String, String)>, carried, merge_into_base, actor }`; `InitialArtifact`/`ArtifactFile` are crate-private now. `CreatedRun.git_refs: Vec<(String, Commit)>` gives the resolved `git_ref` inputs for the creation line. `Project::subject_input(workflow, &inputs)` validates the closure and the values (no Git resolution) and returns the `[subject] from` value. `check_workflow(root, workflow)` takes no initial artifacts. Declared inputs keep their file order (refusal listing, artifact creation order).
- Engine order in `create_run`: workflow exists → freeze (definition refusals first) → input check → `git_ref` resolution → carried artifacts from the previous run → rows. `carried` names must be declared inputs (else `InputProblem::Unknown`); a carried name that is also given is not carried.
- For 04: `surt feature` now only resumes (`resolve_subject`, so a description is an unknown subject) and `--new-run [--base <ref>] [--merge|--no-merge]`. `--new-run` passes `inputs = [("base", <--base>)]` when given (else the workflow's `default` applies) and `carried = ["request"]`, hard-coded in `feature/new_run.rs` with `template::{REQUEST_INPUT, BASE_INPUT, FEATURE_WORKFLOW}`; its started line reads `base` from `CreatedRun.git_refs`. The active-slug refusal of `surt run` already prints `resume: surt continue <subject>`. `merge_flag.rs` and `retry.rs` moved to the crate root (shared by `run` and `feature`); `already_frozen` and `no_active_run` still print `surt feature … --new-run`. `surt run` drives with `SPEC.drive` (command `feature`).
- Engine test fixtures: `ProjectDir::project()` now initializes a Git repository with an empty commit on `main` when there is none (so `git_ref` defaults resolve); `new_run(workflow)` passes `request` only; root test workflows declare `[inputs.request]` and, when they run a base-consuming Operation, `[inputs.base] format = "git_ref" default = "HEAD"`.
- The input refusal is the one refusal with more than two context lines (it lists the inputs); it is tested on its own rather than in `every_refusal`.
