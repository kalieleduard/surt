# 16: Reconcile on every driving command

Spec: [Milestone 1](../spec.md) (stories 71–74, 77–78) · Slice: S5 Recovery and ending · Plan: [I. Recovery](../../../docs/architecture/I-recovery.md#what-a-driving-command-does-when-it-takes-the-lock)

**What to build:** after any interruption, from Ctrl-C to a power cut, the developer just reruns the phase command and it recovers on its own. Inside `drive`, after taking the lock, Surt makes SQLite tell the truth about what already happened (steps 3–6 of I) before scheduling anything new: orphans stopped, unseen endings discarded, half-done integrations settled, the Integration Branch checked.

**Blocked by:** 15 (`surt start` delivers a Feature)

**Status:** resolved

- [x] Process identity is pid + native start time + boot id, implemented in `platform` for the three OSes
- [x] `queued`/`running` Executions of a dead Driver: survivors are stopped (never adopted) and marked `interrupted`; an Execution whose end was never recorded is discarded even if it left a valid delivery
- [x] A pending `ref_move` is confirmed if the ref moved to the target, redone if it didn't move, and blocks if someone else moved it
- [x] Working Copy pending values are promoted when the file matches; a crash between publishing an agent Revision and writing its file is never mistaken for a human edit
- [x] A driving command refuses while the Integration Branch was moved or is checked out, printing the exact `git` command to restore it; steps 3–4 still run and record what they find
- [x] Recovery reports what it found as progress lines, as in I's mock
- [x] `Project` tests for each recovery case, plus a binary test of the moved-branch refusal

## Comments

- 2026-10-18 (orchestrator): Resolved. Merged into `integration/milestone-1` at ec15cbd; code-review fixes at 931fc99.
