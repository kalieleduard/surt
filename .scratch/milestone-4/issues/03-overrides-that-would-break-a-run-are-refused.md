# 03: Overrides that would break a run are refused

Spec: [Milestone 4](../spec.md) (stories 16–21, 27) · Plan: [F. Built-in defaults and local overrides](../../../docs/architecture/F-agent-contract.md#built-in-defaults-and-local-overrides), [F. Effective `agent.toml`](../../../docs/architecture/F-agent-contract.md#effective-agenttoml)

**What to build:** a misspelt directory, a misnamed file or an override that would leave a workflow unable to finish is caught at `surt feature`, with a message naming what is wrong, instead of being ignored or failing hours later. An empty placeholder directory is simply no override.

```
$ surt feature "add login"
✖ .surt/agents/reviewer/agent.toml: outcomes no longer include "changes_requested"
  needed by: task-cycle step review-loop (until)
```

**Blocked by:** 01 (Local `agent.toml` overrides)

**Status:** resolved

- [x] A directory under `.surt/agents/` that is not a built-in Agent id refuses and lists the five known ids
- [x] Inside an Agent directory only `agent.toml`, `instructions.md` and files under `schemas/` are allowed; any other file refuses, naming it
- [x] A local `agent.toml` that sets `id` to anything but its directory name refuses
- [x] An effective profile with `runtime = "fake"` refuses as test-only unless the engine is built with `test-support`
- [x] After merging, the built-in workflow closure is validated against the effective Agents; a removed Outcome, input or output a workflow uses refuses and names the workflow step
- [x] The `Catalog`'s delivery checks are part of that validation: an effective Agent that no longer declares the output a delivery check names, or the Outcome `NoBlockingFinding` names, refuses
- [x] An empty `.surt/agents/` or an empty Agent directory changes nothing
- [x] `Project` tests: each refusal above refuses `create_run` with `InvalidAgentOverride` and writes no row; an empty directory freezes the built-in Agents unchanged
- [x] F records directory hygiene, `fake` outside `test-support` and delivery checks in validation

## Comments

- 2026-10-18 (orchestrator): Implemented and green on Windows. Merged into `integration/milestone-4` at a843a1f; code-review fixes at 863ebc6. The review fixes added the field to `needed by:` (`(until)`, `(when)`, `(input)`, `(delivery check)`) and ignore `.gitkeep`/`.keep` placeholders. Resolved.
