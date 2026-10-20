# 01: An output belongs to Outcomes

Spec: [Milestone 6](../spec.md) (Outputs per Outcome) · Plan: [F. Agent Contract](../../../docs/architecture/F-agent-contract.md), [E](../../../docs/architecture/E-workflow-schema.md)

**What to build:** an Agent with two Outcomes delivers different outputs on each. `[outputs.<name>] outcomes = [...]` names the Outcomes that deliver an output; absent, every Outcome does, so every existing Agent is unchanged.

```toml
outcomes = ["done", "questions"]

[outputs.spec]
format = "markdown"
outcomes = ["done"]

[outputs.spec-questions]
format = "json"
outcomes = ["questions"]
```

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] `OutputSpec` gains `outcomes: Option<Vec<String>>`; it must be a non-empty, unique subset of the Agent's Outcomes, else `InvalidDefinition` with the field path; it is part of the canonical `agent.toml` and of the snapshot identity
- [x] `envelope_schema` lists every output as a property with `additionalProperties = false` and puts in `required` only the outputs every Outcome delivers; no `oneOf`
- [x] `check_names` requires exactly the delivered Outcome's outputs (`MissingOutput` / `UndeclaredOutput`); item files and delivery checks run only for delivered outputs
- [x] A delivery check on an output that the Outcome named by `no_blocking_finding` does not deliver is refused statically
- [x] `AgentLaunch.outputs` and whatever else assumes "all outputs" follow the per-Outcome sets
- [x] Tests through the existing seams: both Outcomes deliver exactly their outputs; missing and extra outputs rejected; the invalid `outcomes` lists refused; the envelope schema's `required`; every existing Agent's behaviour unchanged
- [x] F documents the field and the envelope rule
