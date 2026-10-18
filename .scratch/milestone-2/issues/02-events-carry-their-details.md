# 02: Events carry their details, and the missing kinds are written

Spec: [Milestone 2](../spec.md) (stories 80, 81, 85) · Slice: R1 History · Plan: [D. Events](../../../docs/architecture/D-database.md#events), [I. Intentions](../../../docs/architecture/I-recovery.md#intentions)

**What to build:** every state change's event carries its display details in `event.data`, written in the same transaction as the change, so a timeline can be rendered without joins or guesses. The event kinds D lists for features up to M2 that are never written today are emitted where their state change already happens. The engine never reads `event.data` to decide anything.

Prefactor first: every event write goes through the store's one event-recording helper, which gains a `data` payload. Today a few store modules insert events with their own SQL; they move onto the helper.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Every event insert goes through one helper that takes the kind and its `data`
- [x] `event.data` per kind, at least: Base and actor on `run.created`; artifact, Revision number and origin on `revision.published`; decision, Revision, actor and the reason's first line on `approval.recorded`; pid on `driver.started`/`driver.stopped`; Attempt, slot and runtime on `execution.started`; code and message on `execution.failed`; reason on `step.blocked`; the new budget base on `step.unblocked`; Iteration and cause on `iteration.started`
- [x] `driver.stopped` is written on every Driver exit that releases the lock normally
- [x] `driver.recovered` is written when reconciliation found something, with what it found
- [x] `intention.confirmed` / `intention.discarded` are written when reconciliation settles `ref_move` and `run_branch` Intentions
- [x] `working_copy.edit_refused` is written when a pinned edit or revalidation is refused
- [x] No migration; a Milestone 1 database opens unchanged
- [x] M1's fault-point tests gain assertions on the events written; `Project` tests check `event.data` fields per kind
- [x] D's events section lists the exact field set per kind

## Comments

- 2026-10-18 (orchestrator): Implemented and green on macOS. Merged into `integration/milestone-2` at 40c3305; code-review fixes at 2c96dfe. Resolved.
