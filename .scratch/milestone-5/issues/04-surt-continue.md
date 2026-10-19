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

**Status:** open

- [ ] `surt continue <subject>` finishes a pending cleanup, then drives the active run; no active run refuses suggesting `--new-run`; `--merge`/`--no-merge` without `--new-run` refuse as M4's resume form did
- [ ] `--new-run [--input k=v]… [--merge|--no-merge]` creates the next run of the workflow the latest run used, read from `.surt/`: non-`git_ref` inputs carried unless replaced, `git_ref` inputs resolved again from `--input` or `default`, newly required inputs must be passed
- [ ] `feature`, `plan`, `start`, `phase.rs`, the gate entry checks and `builtin.rs`'s workflow constants are removed
- [ ] `approve`, `reject`, `status` (text and `resume` in JSON), `stop`, interrupted, blocked and integration-branch refusals print `surt continue <subject>`; reject says the producing step runs again
- [ ] CLI suites (`phases`, `feature`, `new_run`, `start`, `merge_flags`, …) move to `run`/`continue`
- [ ] J and A. Process Model replace the phase table with run/continue

## Comments
