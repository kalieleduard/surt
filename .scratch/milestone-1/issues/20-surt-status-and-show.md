# 20: `surt status` and `surt show`

Spec: [Milestone 1](../spec.md) (stories 87–93) · Slice: S5 Recovery and ending · Plan: [J. Reading never writes](../../../docs/architecture/J-cli.md#reading-never-writes)

**What to build:** the developer understands where every Feature stands without opening SQLite, and can read any Revision byte for byte even after cleanup. Reading commands never take a lock, recover, record an edit, recreate a file or write anything, so they are safe in `watch`, scripts and read-only checkouts. Slice S5 is done when the whole test-strategy suite passes.

**Blocked by:** 16 (Reconcile on every driving command), 18 (Cleanup with pending retry)

**Status:** ready-for-agent

- [x] `surt status` lists Features with an active run, their derived state (`RUNNING`, `AWAITING APPROVAL`, `BLOCKED`, `INTERRUPTED`), Driver and progress; `--all` adds terminal runs
- [x] `surt status <feature>` shows the Base, the Integration Branch, each gate's Revision and Approval, and each Task's state, Iteration, Attempt and merged SHA; each blocked step prints its log path and resume command
- [x] At an Approval Gate it lists every file the Approval covers and any unrecorded edit
- [x] It warns when an agent is still running with no Driver, and shows pending integrations and cleanup pending
- [x] `surt show <feature>` lists artifacts; `surt show <feature> <artifact> [--rev N] [--task <key>]` prints the content byte for byte to stdout and warns on stderr when the Working Copy holds an unrecorded edit
- [x] A binary test hashes the database and the Working Copy before and after `status` and `show` and finds them identical
- [ ] The full suite of the test strategy is green on the three OSes
