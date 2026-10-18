# 18: Cleanup with pending retry

Spec: [Milestone 1](../spec.md) (stories 83–86) · Slice: S5 Recovery and ending · Plan: [I. Cleanup](../../../docs/architecture/I-cleanup.md#what-each-ending-removes)

**What to build:** when a run ends, Surt leaves the disk and the ref list clean without losing anything accepted or anything the developer wrote. A cleanup that fails partway stays pending and is finished by the next command that takes the run's lock, so nobody ever cleans up by hand.

**Blocked by:** 15 (`surt start` delivers a Feature)

**Status:** ready-for-agent

- [x] A `succeeded` run removes the Working Copy, execution directories, Task worktrees, Task Branches still at their recorded tip and Salvage Refs; it keeps the Integration Branch, the history and the logs
- [x] A `cancelled` run salvages and removes the worktrees but keeps Task Branches and Salvage Refs
- [x] A Working Copy file with an unrecorded edit, a Task Branch someone moved, or a foreign path is kept with a warning
- [x] Cleanup runs under the lock after the terminal state, is idempotent, and writes `cleaned_at` only when complete
- [x] A removal failure leaves `cleaned_at` NULL and the Feature's next `feature`/`plan`/`start`/`cancel` finishes it
- [x] The E2E of 15 now ends with the cleanup line and asserts the final state: worktrees and Task Branches gone, Integration Branch and logs present, `cleaned_at` written
- [x] `Project` tests: full cleanup, kept edited file, pending cleanup after an injected removal failure and its resumption
