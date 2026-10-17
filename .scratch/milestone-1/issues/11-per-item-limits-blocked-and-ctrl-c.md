# 11: Per-Item, limits, `blocked` and Ctrl-C in `drive`

Spec: [Milestone 1](../spec.md) (stories 44, 59–64, 66–69) · Slice: S3 Engine + Fake · Plan: [G](../../../docs/architecture/G-state-machines.md), [B. Progress and stopping](../../../docs/architecture/B-workspace.md#progress-and-stopping), [ADR 0001](../../../docs/adr/0001-task-cycle-as-child-workflow-run.md)

**What to build:** `drive` runs a `per_item` step as one Child Workflow Run per item, in the order `needs` allows and in parallel up to `max_concurrent_agents`. Limits stop work for a human instead of failing the run, a blocked item holds back only its dependents, and Ctrl-C stops agents gracefully (force on the second signal) without spending Attempts. Tested with synthetic workflows and the Fake through `Project`.

**Blocked by:** 08 (`drive` through agent steps and Approval Gates)

**Status:** ready-for-agent

- [ ] Per-Item expansion commits the Child Workflow Runs and pins in one checkpoint; items start as their `needs` complete
- [ ] `max_concurrent_agents` is a per-Driver semaphore read before each launch; `execution.slot` is written
- [ ] Technical failures retry immediately up to `max_attempts`, then `blocked`; `max_iterations` without completion → `blocked`; `not_retryable` blocks on the first one; a transient failure blocks at once without spending an Attempt and keeps the reset time when given
- [ ] A blocked item holds back only its dependents; independent items keep running; the run never "fails"
- [ ] Rerunning `drive` unblocks every blocked step with a fresh budget while counters keep their history
- [ ] `StopSignal::Graceful` stops agents, lets a running Operation finish and returns `Interrupted`; `Force` kills at once; interrupted Executions spend no Attempt
- [ ] A cancel request written to SQLite is read on the next tick and returns `Cancelled`
- [ ] `Project` tests using the Fake's "waits until stopped" behaviour, with `tokio::time::pause()` and no real sleeps
