# 04: Pure scheduler `decide()`

Spec: [Milestone 1](../spec.md) (stories 59–64, 69) · Slice: S3 Engine + Fake (can start early) · Plan: [G](../../../docs/architecture/G-state-machines.md), [B. The scheduler is a pure function](../../../docs/architecture/B-workspace.md#the-scheduler-is-a-pure-function), [ADR 0008](../../../docs/adr/0008-limits-block-instead-of-failing.md)

**What to build:** the scheduling decision as a pure function, `decide(&RunGraph, &Ctx) -> Vec<Action>`, with the state enums and the actions `StartIteration`, `ExpandItems`, `Launch`, `RunOperation`, `Block`, `FinishRun`, `Return(DriveOutcome)` and `Wait`. Every hard rule of G is proven by table tests, with no I/O, before any loop exists to execute the actions.

**Blocked by:** 01 (Workspace, toolchain and CI matrix)

**Status:** resolved

- [x] `RunGraph`, `Action`, `Ctx` and the state enums import neither `sqlx` nor the `git` module (the DIP exception of [ADR 0012](../../../docs/adr/0012-agent-runtime-is-the-only-trait.md))
- [x] Table tests: `max_attempts` failed Attempts → `blocked`; `max_iterations` Iterations without a gate → `blocked`; a Loop containing an Approval Gate has no Iteration limit
- [x] Table tests: `interrupted` and `transient` Executions spend no Attempt; `not_retryable` blocks on the first occurrence
- [x] Table tests: unblocking renews the budget while keeping counter history; dependents of a blocked item wait while independent ones keep launching
- [x] Table tests: a run only ends `succeeded` or `cancelled`; `Return` is emitted when nothing can run without a human (at a gate, blocked, or done)
- [x] Independent items launch only up to the concurrency limit carried in `Ctx`

## Comments

- 2026-10-18 (orchestrator): Resolved. Merged into `integration/milestone-1` at c3dd0b5; code-review fixes at 931fc99.
