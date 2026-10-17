# 06: Built-in Catalog and `surt feature` creates a Feature

Spec: [Milestone 1](../spec.md) (stories 10–14, 17–18, 21) · Slice: S2 Workflow · Plan: [J. surt feature](../../../docs/architecture/J-cli.md#surt-feature-create-resume-or-start-again), [F](../../../docs/architecture/F-agent-contract.md), [B. Catalog](../../../docs/architecture/B-workspace.md#built-in-definitions-the-catalog)

**What to build:** the `surt` crate embeds the built-in `feature` and `task-cycle` workflows and the five Agents (Spec Agent, Task Agent, Builder, Reviewer, Shipper) in an explicit `Catalog`. `surt feature "<description>"` turns a description into a uniquely named Feature and a `pending` run, then stops without driving. Slice S2 is done when `surt feature "add login"` creates the Subject and a `pending` run.

**Blocked by:** 03 (`surt init` and project discovery), 05 (Workflow parse, validate, freeze and `create_run`)

**Status:** ready-for-agent

- [ ] The `Catalog` is an explicit `include_str!` table: `feature.toml` (no `merge` step, `[cleanup] on_success` only accepts `"clean"`), `task-cycle.toml`, and the five Agents with TOML, instructions and output schemas (`manifest`, `commit`, `feedback`, `delivery`); the engine embeds only the Operation schemas (`base`, `check-report`, `conflict`, `integration`)
- [ ] Every Agent uses the shared profile: `claude-code`, `claude-sonnet-5-5`, effort `high`, auth `subscription`, `dontAsk`, per-role tools as in F, no turn or cost limit
- [ ] The built-in Catalog passes static validation in a test
- [ ] The Feature name is `YYYYMMDD-<slug>` from the local clock via `jiff`; `--name` overrides the slug; a collision gets `-N`; Windows-reserved or non-portable names are rejected or adjusted
- [ ] `--base <ref>` fixes the Base (default `HEAD`)
- [ ] Rerunning with a slug that has an active run refuses and says how to resume or create anew; `--new` skips the guard; after a cancel the same description creates a `-2` Feature
- [ ] Binary tests: create, collision suffix, `--name`, active-slug refusal, `--new`
