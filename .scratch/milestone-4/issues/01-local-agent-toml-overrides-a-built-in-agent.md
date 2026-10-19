# 01: A local `agent.toml` overrides a built-in Agent's profile

Spec: [Milestone 4](../spec.md) (stories 1–8, 14, 15, 22–26, 28–30) · Plan: [F. Built-in defaults and local overrides](../../../docs/architecture/F-agent-contract.md#built-in-defaults-and-local-overrides), [F. Freezing agents](../../../docs/architecture/F-agent-contract.md#freezing-agents), [B. Workspace](../../../docs/architecture/B-workspace.md)

**What to build:** a developer writes a two-line `.surt/agents/builder/agent.toml` that changes the Builder's model and effort, and the next `surt feature` freezes a Builder with exactly that profile. Every other field keeps the built-in value. Every command stops refusing because `.surt/agents/` exists, and a run already created keeps the Agents it froze, whatever happens to the files afterwards.

```
$ surt feature "add login"
✔ created 20261017-add-login   base: main @ a11c9d0
  local agents: builder (agent.toml)
```

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] `Project::create_run` reads `.surt/agents/<id>/agent.toml` for each built-in Agent and merges it over the `Catalog`'s TOML: tables merge recursively by key, an absent field inherits, a scalar or an array replaces the built-in value whole; no deletion syntax
- [x] The merged document is validated against the complete `agent.toml` schema with `deny_unknown_fields` at every level and the existing profile checks (effort syntax, positive finite limits, `api_key_env` only with `auth = "api_key"`), then frozen as the Agent's snapshot
- [x] Any failure is a new `InvalidAgentOverride` refusal carrying the file, the field path (`profile.limits.max_turns`) and the reason; J's message shape, exit `3`; no row is written
- [x] `.surt/agents/` is read only by `create_run`; the `AgentOverrides` refusal on driving commands, its message and its tests are removed
- [x] `create_run` reports which Agents used local files and which; the creation lines of `surt feature "<description>"` and `--new-run` print `local agents: …`, and nothing when there are none
- [x] CRLF and LF versions of the same `agent.toml` freeze to the same snapshot identity
- [x] `Project` tests with the Fake: a Builder override of `model`, `effort` and `[profile.limits]` is frozen as the merged document; every Builder Execution of the run (later Iterations and Attempts included) runs with it even after the file is edited mid-run; an array replaces and a table merges; an unknown field and a bad effort refuse `create_run` with no row written
- [x] Binary tests: the `local agents:` line; an invalid override refusing `surt feature` with exit `3`; `plan` and `start` working with `.surt/agents/` present (replacing the test that expected a refusal)
- [x] F records the creation line; B says `workflow` merges overrides read by `create_run`; K's "Agent overrides" row says it is implemented

## Comments

- 2026-10-18 (orchestrator): Implemented and green on Windows. Merged into `integration/milestone-4` at 9dc553d; code-review fixes at 863ebc6. Resolved.
