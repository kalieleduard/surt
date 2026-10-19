# 07: `status` shows the merge result

Spec: [Milestone 4](../spec.md) (stories 39, 55, 56) · Plan: [J. `status`](../../../docs/architecture/J-cli.md), [A. Merging into the Base](../../../docs/architecture/A-git-integration.md#merging-into-the-base-opt-in)

**What to build:** after a run ends, a developer (or a script deciding whether to open a pull request) can tell from `surt status` whether `main` already has the Feature, and if not, why.

```
$ surt status add-login
20261017-add-login · run 1   SUCCEEDED
  …
  merge    not merged: main moved (a11c9d0 → b22e310)
```

**Blocked by:** 04 (Fast-forward into a Base branch that is not checked out)

**Status:** ready-for-agent

- [ ] The `status` detail keeps the frozen `merge into base` mode and, once the merge step ended, adds the result line: `merged into <ref> <from> → <to>`, or `not merged:` with the reason (`main moved (… → …)`, `no base branch`, `worktree not clean`); nothing for `disabled`
- [ ] `status --json` adds a `merge` object (`outcome`, and `reason`, `from`, `to`, `found` as recorded) to the run document, additively under `"schema": 1`
- [ ] The result is read from the step's recorded `step.done` fields; reads never write
- [ ] Facade test: `status` reports each recorded result
- [ ] Binary tests: the result line and the `merge` JSON object for a merged and a skipped run
- [ ] J records the result line and the JSON field
