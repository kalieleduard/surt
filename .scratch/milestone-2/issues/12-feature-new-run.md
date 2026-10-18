# 12: `surt feature <feature> --new-run`

Spec: [Milestone 2](../spec.md) (stories 49–58, 78) · Slice: R6 New run · Plan: [J. `surt feature`: create, resume or start again](../../../docs/architecture/J-cli.md#surt-feature-create-resume-or-start-again)

**What to build:** a developer can redo a cancelled or delivered Feature under the same name. `surt feature <feature> --new-run` starts run `N+1` of the Subject from its latest `request` Revision, with a fresh Base and freeze, and drives it to the spec Approval Gate. The previous run's branches are never touched, and every read command moves to the new run while `--run 1` keeps the old one readable.

**Blocked by:** 01 (Run selector and `--run`)

**Status:** resolved

- [x] `create_run` gains the "new run of an existing Subject" form: the latest `request` Revision as a new human-origin Initial Artifact, a Base (`HEAD` or `--base`) and a fresh freeze of definitions, agents and configuration
- [x] Branches are `surt/<feature>.N` and `surt/task/<feature>.N/<key>`; worktrees live under `<feature>.N`
- [x] Refuses while the Subject has an active run
- [x] Refuses while the previous run's Working Copy differs from its baseline, listing the files
- [x] The CLI first calls `drive` on the previous root run to finish a pending cleanup, then `create_run`
- [x] `surt feature <feature>` on a Subject with no active run refuses, shows how the last run ended and suggests `--new-run`
- [x] The `cancel` confirmation says that starting over needs `surt feature <feature> --new-run`
- [x] `surt status <feature>` on a Subject with several runs names the run it shows and how many exist
- [x] `Project` tests: `.N` branches and worktree paths, the unrecorded-edit refusal, the active-run refusal
- [x] Binary test: cancel → `--new-run` → spec gate with `surt/<feature>.2`; `status --run 1` still shows the cancelled run

## Comments

- 2026-10-18 (orchestrator): Implemented and green on macOS. Merged into `integration/milestone-2` at a70b135; code-review fixes at 2c96dfe. Resolved.
