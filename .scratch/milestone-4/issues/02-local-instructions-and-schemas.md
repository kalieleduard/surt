# 02: Local instructions and schemas

Spec: [Milestone 4](../spec.md) (stories 9–13) · Plan: [F. Built-in defaults and local overrides](../../../docs/architecture/F-agent-contract.md#built-in-defaults-and-local-overrides), [F. Instructions and assembled context](../../../docs/architecture/F-agent-contract.md#instructions-and-assembled-context)

**What to build:** a developer gives the Reviewer the project's conventions with a local `instructions.md`, and tightens what it must deliver with a local schema, without copying the rest of the built-in Agent. Schemas they did not override stay in effect, and freezing a run never touches the network.

**Blocked by:** 01 (Local `agent.toml` overrides)

**Status:** resolved

- [x] A local `instructions.md` replaces the whole built-in instruction text of that Agent, is frozen with it, and is the text the Context Assembler hands to the runtime
- [x] A schema path in the effective `agent.toml` resolves to the local file under the agent's directory when it exists, otherwise to the built-in asset of the same path; a path found in neither refuses with `InvalidAgentOverride`
- [x] Local schemas' `$ref`s are followed transitively within the same agent directory with the existing local-reference rules, and the closure is frozen; a network or absolute reference refuses
- [x] `Project` tests with the Fake: the assembled context carries the local instructions; a local schema overlays the built-in one and a delivery that violates it is rejected while one that satisfies it is accepted; a schema split with a local `$ref` freezes and validates; a missing schema and a network `$ref` refuse `create_run` with no row written
- [x] The creation line lists `instructions.md` and `schemas/…` next to `agent.toml`
- [x] F records the schema resolution order (local first, then built-in)

## Comments

- 2026-10-18 (orchestrator): Implemented and green on Windows. Merged into `integration/milestone-4` at 4acec9e; code-review fixes at 863ebc6. Resolved.
