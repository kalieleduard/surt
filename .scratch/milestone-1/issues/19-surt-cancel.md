# 19: `surt cancel`

Spec: [Milestone 1](../spec.md) (stories 79–82) · Slice: S5 Recovery and ending · Plan: [J. Stop and cancel](../../../docs/architecture/J-cli.md#stop-and-cancel), [I. Without a Driver](../../../docs/architecture/I-recovery.md#without-a-driver-surt-cancel-and-surt-status)

**What to build:** the developer can end a run from any terminal with one command: crashed runs are settled and cancelled directly, and a run driven elsewhere is told to stop through SQLite. A confirmation guards against accidents, and scripts never hang on a prompt.

**Blocked by:** 16 (Reconcile on every driving command), 18 (Cleanup with pending retry)

**Status:** resolved

- [x] `surt cancel` shows what will be stopped and what stays, and asks for confirmation; the CLI asks, the engine's `cancel` acts when called
- [x] Without a TTY and without `--yes`, it refuses with exit `3`
- [x] With no live Driver: takes the lock, stops orphans, settles pending Intentions so history says where the branch really is, records the Cancellation, then runs the `cancelled` cleanup
- [x] With a live Driver: writes the cancel request; that Driver stops and exits `5`
- [x] Binary tests: refusal without TTY, cancel of a crashed run, cancel across two processes with exit `5`; afterwards `surt feature "<same description>"` creates a `-2` Feature

## Comments

- 2026-10-18 (orchestrator): Resolved. Merged into `integration/milestone-1` at fe31360; code-review fixes at 931fc99.
