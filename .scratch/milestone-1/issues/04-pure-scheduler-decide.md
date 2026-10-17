# 04: Pure scheduler `decide()`

Spec: [Milestone 1](../spec.md) (stories 59–64, 69) · Slice: S3 Engine + Fake (can start early) · Plan: [G](../../../docs/architecture/G-state-machines.md), [B. The scheduler is a pure function](../../../docs/architecture/B-workspace.md#the-scheduler-is-a-pure-function), [ADR 0008](../../../docs/adr/0008-limits-block-instead-of-failing.md)

**What to build:** the scheduling decision as a pure function, `decide(&RunGraph, &Ctx) -> Vec<Action>`, with the state enums and the actions `StartIteration`, `ExpandItems`, `Launch`, `RunOperation`, `Block`, `FinishRun`, `Return(DriveOutcome)` and `Wait`. Every hard rule of G is proven by table tests, with no I/O, before any loop exists to execute the actions.

**Blocked by:** 01 (Workspace, toolchain and CI matrix)

**Status:** ready-for-agent

- [ ] `RunGraph`, `Action`, `Ctx` and the state enums import neither `sqlx` nor the `git` module (the DIP exception of [ADR 0012](../../../docs/adr/0012-agent-runtime-is-the-only-trait.md))
- [ ] Table tests: `max_attempts` failed Attempts → `blocked`; `max_iterations` Iterations without a gate → `blocked`; a Loop containing an Approval Gate has no Iteration limit
- [ ] Table tests: `interrupted` and `transient` Executions spend no Attempt; `not_retryable` blocks on the first occurrence
- [ ] Table tests: unblocking renews the budget while keeping counter history; dependents of a blocked item wait while independent ones keep launching
- [ ] Table tests: a run only ends `succeeded` or `cancelled`; `Return` is emitted when nothing can run without a human (at a gate, blocked, or done)
- [ ] Independent items launch only up to the concurrency limit carried in `Ctx`
