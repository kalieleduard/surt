# 04: Agent Slots as project-wide lock files

Spec: [Milestone 3](../spec.md) (stories 26–28, 35–39) · Plan: [A. Agent concurrency limit](../../../docs/architecture/A-process-model.md#agent-concurrency-limit), [ADR 0002](../../../docs/adr/0002-file-locks-over-sqlite-leases.md)

**What to build:** a developer driving two Features of one project in two terminals never runs more agents than `max_concurrent_agents`. Each Agent Slot is a lock file `.surt/runtime/slots/agent-<i>.lock`. Before launching, every Driver of the project takes a free slot with `try_lock`. It holds the slot until that agent Execution ends, then releases it. A Driver that finds every slot taken keeps its agents queued and retries on its next tick. A Driver killed with `kill -9` frees its slots through the OS. Projects never share slots.

**Blocked by:** None (can start immediately). It changes the same Driver loop as 02, so landing it after 02 avoids a merge.

**Status:** resolved

- [x] Slots use the same separate-file lock primitive as the run locks, on macOS, Linux and Windows; the slots directory is created on first use; lock files are never deleted, and `surt delete` leaves them alone
- [x] On each tick the Driver `try_lock`s the slot indices below the limit that it does not hold; the slots it gets are the free slots passed to `decide()`, which already gives one slot to each `Launch`
- [x] A slot taken for a launch is held until that Execution's flight lands; slots tried but not used are released in the same tick
- [x] With no free slot, ready agents stay queued and the Driver retries on its next tick (about one second); queued agents of one Driver go in deterministic order (manifest order for Tasks)
- [x] `execution.slot` records the index, for display only; recovery does not read it
- [x] The footer's `slots N/M` is `M` minus the Driver's free slots, so it includes other Drivers
- [x] The per-Driver semaphore is gone, and with it B's rule exception for `Flights` counting slots inside one Driver
- [x] `Project::drive` test: two `Project` handles on one project directory drive two Features with `max_concurrent_agents = 1` and Fake Executions that wait until stopped; two agents never run at once; dropping one Driver lets the other launch
- [x] Binary test: two `surt start` processes on two Features with a limit of 1 never have two agent Executions running at once
- [x] A records the tick cadence in place of "~500 ms"; K's "Agent slots across processes" row says it is implemented

## Comments

- 2026-10-18 (orchestrator): Implemented and green on Windows. Merged into `integration/milestone-3` at 8c4b8b3; code-review fixes at 5e84027. Resolved.
