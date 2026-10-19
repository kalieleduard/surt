# 04: `surt continue` replaces the phases

Spec: [Milestone 5](../spec.md) (stories 23–26) · Plan: [J](../../../docs/architecture/J-cli.md), [A. Process Model](../../../docs/architecture/A-process-model.md)

**What to build:** one generic command drives any run again. `surt feature <subject>`, `surt plan`, `surt start` and the CLI's phase table disappear; every "next" line points at `surt continue <subject>`.

```
$ surt approve add-login
✔ spec rev 1 approved
  next: surt continue add-login
$ surt continue add-login --new-run --input base=develop
```

**Blocked by:** 03

**Status:** resolved

- [x] `surt continue <subject>` finishes a pending cleanup, then drives the active run; no active run refuses suggesting `--new-run`; `--merge`/`--no-merge` without `--new-run` refuse as M4's resume form did
- [x] `--new-run [--input k=v]… [--merge|--no-merge]` creates the next run of the workflow the latest run used, read from `.surt/`: non-`git_ref` inputs carried unless replaced, `git_ref` inputs resolved again from `--input` or `default`, newly required inputs must be passed
- [x] `feature`, `plan`, `start`, `phase.rs`, the gate entry checks and `builtin.rs`'s workflow constants are removed
- [x] `approve`, `reject`, `status` (text and `resume` in JSON), `stop`, interrupted, blocked and integration-branch refusals print `surt continue <subject>`; reject says the producing step runs again
- [x] CLI suites (`phases`, `feature`, `new_run`, `start`, `merge_flags`, …) move to `run`/`continue`
- [x] J and A. Process Model replace the phase table with run/continue

## Comments

- Resolved on `m5/04-surt-continue`. `crates/surt/src/continue_run.rs` (+ `continue_run/new_run.rs`) holds `surt continue`, `continue_command`/`new_run_command` (every next/resume/retry line and every `--new-run` hint goes through them), `finish_cleanup` and `active_run`. `Driven` lost its `command` field. `run/inputs.rs` is `pub(crate)` and shared by `run` and `continue --new-run`; `--input` has `requires = "new_run"` (exit `2` otherwise).
- Engine: `NewRun.carried: Vec<String>` became `NewRun.carry: Carry` (`Nothing` | `PreviousInputs`). With `PreviousInputs` the engine carries every declared non-`git_ref` input the latest root run has as an artifact, unless given; `git_ref` inputs are resolved from `--input` or `default`; a Subject without runs refuses `UnknownSubject` before the input check. `Project::latest_workflow(subject)` reads the workflow name from the latest run's frozen definition. `Decided.producer: Option<String>` is the key of the Agent step whose Agent declares the gate's artifact as an output (`Frozen::producer`, also used by the Working Copy's `producer_rule`). `template::{FEATURE_WORKFLOW, REQUEST_INPUT, BASE_INPUT}` now live in the Template's tests only.
- Behaviour choices: a plain `continue` that only finishes a pending cleanup exits `0` (as `plan`/`start` did); a second `continue` then refuses with `start run N+1 with the same inputs: surt continue <subject> --new-run`. `--merge`/`--no-merge` on a plain `continue` refuse with `already_frozen` before the cleanup. The new-run started line is `run N of <subject>   <input>: <ref> @ <sha>…   branch surt/<subject>.N`. `reject` says `<producer> will redo it reading your reason` (fallback `the step that produced it`); the item-file listing after a reject (`(with tasks/A.md … tasks/C.md)`) was dropped, since `Decided` carries no Item List knowledge.
- For 05: removing `phase.rs` forced two interim generic rules in `status`: gate rows are labelled by the gate's artifact (`manifest rev 1 …`, no `tasks` label), and a gate whose Revision holds more than one file counts `files - 1` item files (`status::item_files`), still worded "task files"/"tasks". Replace that count with a real Item List rule from the engine and the wording with items. The items section title is a local `TASK_STEP = "implement"` const in `status/detail/tasks.rs`; title it by the `per_item` step key. `continue_run/new_run.rs::current` still uses `StatusScope::Feature`. CLI suites: `phases.rs` → `continuing.rs`, `start.rs` → `delivering.rs`, new `new_run_inputs.rs`; `Sandbox::feature` → `Sandbox::continue_run`. Test names still say feature/task in places.
- For 06: `base.schema.json`'s title is already generic (`Base (a git_ref run input resolved to a commit)`, engine asset and `docs/architecture/schemas/` copy) so `rg "surt (feature|plan|start)\b" crates` is clean. `.surt/features/` paths in messages and the `branch surt/<subject>.N` text in the started line are untouched.
- For 07: only J and A. Process Model were updated; A-git-integration, A-overview, B, C, D, E, F, G, H, I-cleanup, I-recovery, K and `docs/smoke-claude-code.md` still mention `surt feature`/`plan`/`start`.
