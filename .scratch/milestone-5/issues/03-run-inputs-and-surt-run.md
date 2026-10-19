# 03: A workflow declares its run inputs; `surt run` supplies them

Spec: [Milestone 5](../spec.md) (stories 12–17, 21, 22, 25) · Plan: [E](../../../docs/architecture/E-workflow-schema.md), [J](../../../docs/architecture/J-cli.md)

**What to build:** `[inputs.<name>]` and `[subject]` in a root workflow say what a run needs and where its name comes from. `surt run <workflow> --input name=value` creates the run and drives it to the next gate, replacing the creation form of `surt feature "<description>"`.

```
$ surt run feature --input request="add login" --input base=main --merge
✔ created 20261019-add-login   workflow: feature   base: main @ a11c9d0
  merge into base: ff-only (--merge)
```

**Blocked by:** 02

**Status:** open

- [ ] Workflow schema: `[inputs.<name>]` with `format = "text" | "markdown" | "json" | "git_ref"`, `optional`, `default` (`git_ref` only), `description`; `[subject] from = "<text or markdown input>"`; a `per_item` child declaring inputs is refused; both are in the canonical form
- [ ] Declared inputs are the initial artifact names of static validation; an artifact consumed by a Git Operation must come from a `git_ref` input, otherwise refused
- [ ] `NewRun` carries input values; the engine builds the artifacts (`<name>.txt|.md|.json`, `git_ref` → `<name>.json` `{ref, sha}`), validating format, required, unknown and duplicate inputs and resolving `git_ref` (with `default`) before any row is written
- [ ] Slug: `--name`, else `[subject] from`'s value, else the workflow name
- [ ] `surt run <workflow> [--input k=v | k=@path]… [--name] [--new] [--merge|--no-merge]`; an active Subject with the same slug refuses pointing at `surt continue <subject>` unless `--new`; input refusals list every declared input with format, optional and description
- [ ] The creation line prints `workflow:` and each `git_ref` input; `surt feature "<description>"` (creation) is removed; resume forms of `feature`, `plan` and `start` stay until 04
- [ ] The feature Template declares `request` (`text`) and `base` (`git_ref`, default `HEAD`) and `[subject] from = "request"`; init's `next:` line prints `surt run feature --input request="<what you want to build>"`
- [ ] Tests: each format, `@file`, defaults, every refusal, `[subject] from`, a workflow with no inputs, a `base`-consuming Operation without a `git_ref` input; CLI creation tests move to `surt run`
- [ ] E documents `[inputs]`, `[subject]` and the Operations' artifact contract; J documents `surt run`

## Comments
