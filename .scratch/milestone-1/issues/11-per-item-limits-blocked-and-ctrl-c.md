# 11: Per-Item, limits, `blocked` and Ctrl-C in `drive`

Spec: [Milestone 1](../spec.md) (stories 44, 59–64, 66–69) · Slice: S3 Engine + Fake · Plan: [G](../../../docs/architecture/G-state-machines.md), [B. Progress and stopping](../../../docs/architecture/B-workspace.md#progress-and-stopping), [ADR 0001](../../../docs/adr/0001-task-cycle-as-child-workflow-run.md)

**What to build:** `drive` runs a `per_item` step as one Child Workflow Run per item, in the order `needs` allows and in parallel up to `max_concurrent_agents`. Limits stop work for a human instead of failing the run, a blocked item holds back only its dependents, and Ctrl-C stops agents gracefully (force on the second signal) without spending Attempts. Tested with synthetic workflows and the Fake through `Project`.

**Blocked by:** 08 (`drive` through agent steps and Approval Gates)

**Status:** resolved

- [x] Per-Item expansion commits the Child Workflow Runs and pins in one checkpoint; items start as their `needs` complete
- [x] `max_concurrent_agents` is a per-Driver semaphore read before each launch; `execution.slot` is written
- [x] Technical failures retry immediately up to `max_attempts`, then `blocked`; `max_iterations` without completion → `blocked`; `not_retryable` blocks on the first one; a transient failure blocks at once without spending an Attempt and keeps the reset time when given
- [x] A blocked item holds back only its dependents; independent items keep running; the run never "fails"
- [x] Rerunning `drive` unblocks every blocked step with a fresh budget while counters keep their history
- [x] `StopSignal::Graceful` stops agents, lets a running Operation finish and returns `Interrupted`; `Force` kills at once; interrupted Executions spend no Attempt
- [x] A cancel request written to SQLite is read on the next tick and returns `Cancelled`
- [ ] `Project` tests using the Fake's "waits until stopped" behaviour, with `tokio::time::pause()` and no real sleeps

## Comments

- 2026-10-18 (orchestrator): Resolved. The `tokio::time::pause()` box stays unticked by design: pausing the clock makes sqlx's pool acquire time out at once; tests use the Fake's wait-until-stopped behaviour and event wakeups instead. Recorded in `docs/architecture/B-workspace.md` "Rule exceptions". Merged into `integration/milestone-1` at 3573d05; code-review fixes at 931fc99.
