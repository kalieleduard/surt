# 03: Full `runtime_info` from Claude Code and the Fake

Spec: [Milestone 2](../spec.md) (stories 4, 82) · Slice: R1 History · Plan: [D. Executions](../../../docs/architecture/D-database.md#executions)

**What to build:** each Execution records what the runtime reported about it, so `inspect` can tell an expensive or stuck agent from a cheap one. The Claude Code adapter writes the full `runtime_info` at the end of an Execution; the Fake writes the subset it simulates. Values the runtime did not report are absent, never `0`.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [x] Claude Code: version, session id, reported model, requested effort, exit code, turns, usage, cost and permission denials, each only when reported
- [x] The Fake records the fields it simulates, so engine and binary tests can assert on them
- [x] Fixture tests over the existing captured streams (success, auth failure, nonexistent model, SIGTERM) check what is extracted and what is absent
- [x] An Execution that ended before a result event (killed, SIGTERM) still records what was known
