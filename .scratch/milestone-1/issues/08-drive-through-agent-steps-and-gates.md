# 08: `drive` through agent steps and Approval Gates

Spec: [Milestone 1](../spec.md) (stories 16, 24–29, 45, 70, 101) · Slice: S3 Engine + Fake · Plan: [B. Project](../../../docs/architecture/B-workspace.md#the-engines-interface-project), [H](../../../docs/architecture/H-artifact-storage.md), [A. Process Model](../../../docs/architecture/A-process-model.md)

**What to build:** `Project::drive` advances a run until nothing can run without a human: it takes the run lock, reloads the `RunGraph` every tick, executes the actions of `decide`, launches agents through the Fake, publishes their Revisions and stops at an Approval Gate. `approve` and `reject` record decisions without driving, and a rejection starts a new Iteration whose agent reads the rejected Revision and the reason. Tested with synthetic workflows through the `Project` facade.

**Blocked by:** 04 (Pure scheduler `decide()`), 07 (AgentRuntime trait, Fake Runtime and delivery validation)

**Status:** resolved

- [x] One Driver per root run via a file lock on the run; a second `drive` refuses with the live Driver's pid; dropping the process releases the lock
- [x] Each tick reloads the `RunGraph`, so an Approval recorded by another process is seen without caching
- [x] Revisions are SHA-256 blobs in SQLite with atomic publication; an Approval stores id + hash and targets the gate's latest Revision; `--rev N` mismatch is refused
- [x] The Context Assembler writes per-Execution inputs with labels (Iteration and producing step) and, from the second Iteration on, why the Iteration started; every Execution is a fresh session with only its fixed Inputs
- [x] Each Execution's stream and stderr go to the Feature's log directory under `.surt/logs/`, which survives cleanup
- [x] The engine never prints: it emits typed `Progress` events; outcomes map to `DriveOutcome` and errors to `Refused`/`Internal`
- [x] `Project` tests: agent → gate (`AtGate`), reject → Iteration 2 reads reason and rejected Revision → approve → next step; a second Driver refuses

## Comments

- 2026-10-18 (orchestrator): Resolved. Merged into `integration/milestone-1` at 66e71ba; code-review fixes at 931fc99.
