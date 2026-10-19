# 01: Run selector and `--run` on `status` and `show`

Spec: [Milestone 2](../spec.md) (stories 18, 19, 79) · Slice: R1 History · Plan: [J. Referring to features, runs, tasks and Executions](../../../docs/architecture/J-cli.md#referring-to-features-runs-tasks-and-executions)

**What to build:** a developer can read an earlier run of a Subject. Read methods take a run selector (the active run, else the latest one, or ordinal `N`), resolved in the engine next to Subject resolution. `surt status <feature> --run N` and `surt show <feature> <artifact> --run N` use it. Every later read command (`inspect`, `logs`) reuses the same selector.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Read methods of `Project` accept a run selector; the default is the active run, else the latest run of the Subject
- [x] `--run N` on `status` and `show` reads run `N`
- [x] `--run N` for a run that doesn't exist is a `Refusal` carrying the Subject's runs with their states; the CLI lists them and exits `3`
- [x] The refusal carries observed values only; the CLI owns the wording
- [x] Binary tests: `--run 1` on a single-run Subject, an unknown `N` refusing with the list; the "reads never write" hash check covers `--run`

## Comments

- 2026-10-18 (orchestrator): Implemented and green on macOS. Merged into `integration/milestone-2` at 081f8db; code-review fixes at 2c96dfe. Resolved.
