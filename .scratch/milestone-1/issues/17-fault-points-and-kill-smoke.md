# 17: Fault points at the eight checkpoints and the `kill` smoke

Spec: [Milestone 1](../spec.md) (story 116) · Slice: S5 Recovery and ending · Plan: [I. Transactional checkpoints](../../../docs/architecture/I-recovery.md#transactional-checkpoints), [Test strategy](../../surt-architecture/issues/16-test-strategy.md)

**What to build:** proof that a crash between any Intention and its effect is recovered: a fault can be injected right after each of the eight transactional checkpoints, and a real process `kill` during a waiting Execution is resumed to completion.

**Blocked by:** 16 (Reconcile on every driving command)

**Status:** ready-for-agent

- [ ] Fault points exist only with the `test-support` feature and can fail the process right after the commit of: run created, Iteration started / Per-Item expansion, Execution started, process spawned, Intention, Execution accepted, Working Copy written, Execution ended otherwise
- [ ] For each checkpoint, a `Project` test fails there and checks the next `drive` finishes the run without duplicating any effect (no double merge, no duplicate Revision, no lost Approval)
- [ ] The `kill` smoke kills the `surt` process during a Fake Execution that waits until stopped; resuming finishes the run, with the killed Execution `interrupted` and a new Attempt
- [ ] A `kill -9` of a Driver releases the run lock and the next command takes over
- [ ] All tests are deterministic on the three OSes, with no real sleeps
