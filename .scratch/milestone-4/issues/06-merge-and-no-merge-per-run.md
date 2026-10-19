# 06: `--merge` and `--no-merge` per run

Spec: [Milestone 4](../spec.md) (stories 32–38) · Plan: [J. `surt feature`](../../../docs/architecture/J-cli.md), [D. Snapshots](../../../docs/architecture/D-database.md#snapshots)

**What to build:** a developer opts one Feature into the fast-forward with `--merge`, or keeps a risky one off `main` with `--no-merge`, without touching `.surt/config.toml`. The choice is frozen with the run, shown when the run is created, and a flag that could not take effect is refused instead of ignored.

```
$ surt feature "add login" --merge
✔ created 20261017-add-login   base: main @ a11c9d0
  merge into base: ff-only (--merge)

$ surt feature 20261017-add-login --no-merge
✖ run 1 of 20261017-add-login already froze merge_into_base = "ff-only"
  to choose again, start a new run: surt feature 20261017-add-login --new-run --no-merge
```

**Blocked by:** 04 (Fast-forward into a Base branch that is not checked out)

**Status:** ready-for-agent

- [ ] `--merge` and `--no-merge` exist on `surt feature`, mutually exclusive in clap (exit `2`)
- [ ] They apply on the description form and on `--new-run`; `NewRun` carries the override and `create_run` freezes the effective value (`ff-only` / `never`) into the `config` snapshot; the engine knows nothing of the flags
- [ ] On the resume form either flag refuses with exit `3`, naming the frozen value and pointing to `--new-run`
- [ ] The creation line prints `merge into base: <value>` and, when a flag chose it, which one; nothing extra prints for the project default `never` without a flag
- [ ] `Project` test: a per-run override frozen in `config` wins over the project file, and a later edit of the file changes nothing for that run
- [ ] Binary tests: both flags on creation and on `--new-run`; both together exit `2`; either on resume exits `3`; the creation line
- [ ] J records the flags on `--new-run`, their refusal on resume and the creation line
