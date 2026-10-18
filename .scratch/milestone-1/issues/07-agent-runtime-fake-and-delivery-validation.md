# 07: AgentRuntime trait, Fake Runtime and delivery validation

Spec: [Milestone 1](../spec.md) (stories 113–114) · Slice: S3 Engine + Fake · Plan: [F. Agent Runtime and Fake Runtime](../../../docs/architecture/F-agent-contract.md#agent-runtime-and-fake-runtime), [ADR 0012](../../../docs/adr/0012-agent-runtime-is-the-only-trait.md)

**What to build:** the only trait in the engine, `AgentRuntime` (native `async`, not object-safe), which launches, follows and stops an Execution, plus a `FakeRuntime` that drives every scenario without tokens. One delivery validator is shared by the Fake, the future Claude Code runtime and human Item List edits, so a Fake delivery is accepted or refused exactly as a real one would be.

**Blocked by:** 05 (Workflow parse, validate, freeze and `create_run`)

**Status:** ready-for-agent

- [x] `RuntimeError` carries a `FailureClass` (`transient`, `not_retryable`, `retryable`) classified by the adapter; the runtime does not resolve inputs, publish Revisions or write to SQLite
- [x] `FakeRuntime` and its typed script (`Script::new().on(<step key>, <iteration>, <attempt>, Behavior::…)`) are public but only with the `test-support` feature
- [x] Fake behaviours: valid delivery, invalid JSON, missing file, model mismatch, crash, limit failure (transient, with optional reset time), and an Execution that waits until stopped
- [x] Delivery validation: envelope `{outcome, outputs}`, relative paths with no escape or link, formats and JSON Schema draft-07, plus consistency checks registered by the workflow
- [x] Tests cover each Fake behaviour against the validator, including every rejection reason
