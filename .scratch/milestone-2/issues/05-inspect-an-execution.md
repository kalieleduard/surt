# 05: `surt inspect <execution-id>`

Spec: [Milestone 2](../spec.md) (stories 3, 4, 6) · Slice: R1 History · Plan: [J. `surt inspect`](../../../docs/architecture/J-cli.md#surt-inspect), [J. Referring to … Executions](../../../docs/architecture/J-cli.md#referring-to-features-runs-tasks-and-executions)

**What to build:** one screen tells a developer everything about one agent run. An Execution is referenced by its full id or by a unique prefix of at least 8 characters, as it appears in Salvage Refs, Execution directories and `surt` output. The detail shows role, Task, step, Iteration and Attempt, state and failure, profile (runtime, model, effort, permission mode), session id, pid, exit code, duration, the reported model, turns, usage, cost and permission denials when reported, the Inputs it read (with Revision numbers and SHA-256), its worktree, its Salvage Ref and how to read its log.

**Blocked by:** 03 (Full `runtime_info`), 04 (`surt inspect` timelines)

**Status:** resolved

- [x] Execution reference resolution lives in the engine next to Subject resolution: full id or unique prefix of 8+ characters
- [x] A prefix shorter than 8 or matching several Executions is a `Refusal` with candidates; the CLI lists them and exits `3`
- [x] Values not reported by the runtime are left out, never printed as `0`
- [x] The Salvage Ref is read with a read-only `git for-each-ref`; nothing else outside SQLite is needed
- [x] The log line names `surt logs <id>` (the command lands in 06; the hint is just text)
- [x] Binary tests: full id, 8-character prefix, too-short and ambiguous prefixes, an Execution after cleanup

## Comments

- 2026-10-18 (orchestrator): Implemented and green on macOS. Merged into `integration/milestone-2` at fda8f1b; code-review fixes at 2c96dfe. Resolved.
