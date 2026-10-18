# 02: A Transient Failure pauses agent launches instead of blocking

Spec: [Milestone 3](../spec.md) (stories 1–11, 14–18, 23–25) · Plan: [I. Transient Failures](../../../docs/architecture/I-recovery.md#transient-failures-network-and-rate-limits), [G. Limits](../../../docs/architecture/G-state-machines.md#limits), [B. Workspace](../../../docs/architecture/B-workspace.md)

**What to build:** a developer leaving `surt start` running overnight no longer comes back to a Task blocked by a rate limit. When an agent Execution fails `transient` (rate limit, overload, server or network error), the Driver pauses **every agent launch** until the reset time the stream reported. Without a reset time, it waits with a backoff of 1, 2, 4, 8 and then 15 minutes. Running agents go on, Operations (Checks, integrations) keep running, no Attempt is spent, and there is no ceiling. Then launches resume and the Step Run gets its next Attempt. Ctrl-C or `surt stop` during the pause is an ordinary resumable Interruption.

```
✖ task B build attempt 2: rate limited (resets 03:00)
⏸ agent launches paused until 03:00 — 2 waiting · Ctrl-C to stop (resumable)
↻ agent launches resumed
↻ task B build attempt 2
```

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] `decide()` no longer blocks a Step Run whose last Execution failed `transient`; `not_retryable` still blocks at once, `retryable` still spends an Attempt, and an unknown category stays `retryable`
- [ ] The scheduler context takes `paused_until` and `now`; while paused, `decide()` emits no agent `Launch` but still emits `RunOperation`; it stays free of `sqlx` and `git`
- [ ] A pure pause-schedule function taking `now`: a future reset ends the pause at that time; with several resets, the latest one wins; a reset in the past counts as no reset; without a reset the backoff runs 1, 2, 4, 8, then 15 min; the step advances only when a resume fails, not for failures of agents already running; it resets after an agent Execution ends any other way
- [ ] No probe launch: when the pause ends, launches resume up to the free slots
- [ ] The Driver loop waits for the earliest of a landed flight, its tick, a raised stop level or the end of the pause; a stop during the pause ends `Interrupted` (exit `130`); a new driving command starts unpaused
- [ ] `agents.paused` (`until`, `code`, `backoff_step` when backing off, `waiting`) is written when launches pause and again when the pause end moves later; `agents.resumed` (`paused_seconds`) when they resume; D's field table lists them
- [ ] `Progress` gains `Paused { until }` and `Resumed`; the CLI prints the paused and resumed lines through the symbol table (`⏸`/`[paused]`, `↻`/`[retry]`)
- [ ] A run that an older binary blocked on a Transient Failure resumes with the usual fresh budget, with no data repair
- [ ] `decide()` table tests: paused with no launch while Operations run, unpaused launches, `transient` no longer blocks (replacing the M1 rows that expected a block)
- [ ] Pause-schedule table tests for every rule above
- [ ] `Project::drive` tests with the Fake (`limit_failure` with a reset a few seconds ahead): pause then resume with no Attempt spent and the run `succeeded`; no agent starts during the pause while a ready Operation runs; an agent already running finishes and is accepted; a graceful stop during the pause interrupts, and driving again launches at once
- [ ] The binary test that expected a rate-limited Task to block with exit `4` now expects the pause and resume lines and the Task merged
- [ ] I records the "decided here" points (latest reset wins, a past reset counts as none, the step advances per failed resume, no probe); G's Transient Failure line matches; K's "Pause on Transient Failure" row says it is implemented
