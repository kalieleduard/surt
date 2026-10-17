# 05: Workflow parse, validate, freeze and `create_run`

Spec: [Milestone 1](../spec.md) (story 15) · Slice: S2 Workflow · Plan: [E](../../../docs/architecture/E-workflow-schema.md), [D. When rows are created](../../../docs/architecture/D-database.md#when-rows-are-created)

**What to build:** the engine accepts workflow definitions, rejects invalid ones with precise errors, and creates a run that freezes the closure of definitions, agents and project configuration so later edits never affect it. Everything is exercised through `Project::create_run` with synthetic workflows; the engine never sees `feature.toml`.

**Blocked by:** 02 (Store opens the schema)

**Status:** ready-for-agent

- [ ] The parser accepts the five `kind`s (`agent`, `approval`, `operation`, `loop`, `per_item`) with `deny_unknown_fields`, without expressions; conditions only compare Outcomes; `needs` is explicit
- [ ] Static validation checks every reference against the supplied agents and the Operations registry, and reports the offending step
- [ ] `create_run` commits the "run created" checkpoint in one transaction: Subject, root run `pending`, frozen snapshots with SHA-256 identity, Initial Artifacts and top-level Step Runs `pending`
- [ ] Editing the config or definitions after `create_run` does not change the frozen snapshot of that run
- [ ] A second active root run for the same Subject is refused
- [ ] `Project` tests with valid and invalid synthetic definitions (unknown field, unknown kind, dangling reference, cycle in `needs`)
