# 13: `surt delete`

Spec: [Milestone 2](../spec.md) (stories 59–70) · Slice: R7 Delete · Plan: [I. `surt delete`](../../../docs/architecture/I-cleanup.md#surt-delete), [D. Deleting history](../../../docs/architecture/D-database.md#deleting-history), [ADR 0009](../../../docs/adr/0009-history-is-purged-not-soft-deleted.md)

**What to build:** a developer can remove an old Feature for good without losing delivered work. `surt delete <feature>` shows the impact, asks `[y/N]`, and purges every run's worktrees, Task Branches, Salvage Refs, the Integration Branches of runs that didn't succeed, logs, Working Copy, Execution directories and every SQLite row. A delete interrupted by a Git or disk failure is finished by running it again.

**Blocked by:** 12 (`--new-run`), which the test needs for a Subject with a cancelled and a succeeded run

**Status:** ready-for-agent

- [ ] A delete impact read companion: runs by state, refs to delete and to keep, directories with sizes, unrecorded edits, Revision/Execution/event counts
- [ ] The CLI shows it and asks `[y/N]`; `--yes` skips it; without a TTY and without `--yes` it refuses with exit `3`
- [ ] An active run refuses with exit `3` and `fix: surt cancel <feature>`; `delete` never stops agents
- [ ] Step 1: `try_lock` every root run lock of the Subject; any busy lock refuses with exit `3`
- [ ] Step 2, each part idempotent: worktrees unlocked and force-removed (a missing entry pruned); one `update-ref --stdin` transaction deleting Task Branches, Salvage Refs and non-delivered Integration Branches at whatever SHA they hold; the log, Working Copy and Execution directories removed
- [ ] A failure in step 2 finishes the rest of the step, leaves the database untouched, exits `1` and prints `retry: surt delete <feature>`
- [ ] Step 3: one `BEGIN IMMEDIATE` transaction rereads the root runs, rolls back and refuses if they changed, then deletes the Subject and garbage-collects unreferenced snapshots and blobs
- [ ] Step 4: locks released and lock files removed, best effort
- [ ] The Integration Branch of every `succeeded` run is kept; a `cancelled` run's is deleted
- [ ] Never touches the Base, the user's own worktrees or any path Surt didn't record
- [ ] A kept run's status line says `remove with: surt delete <feature>`
- [ ] `Project` tests: busy lock refuses; a removal failure leaves rows intact and the rerun finishes; a run set that changed between step 2 and the purge rolls back; no unreferenced snapshot or blob remains; delivered Integration Branches survive
- [ ] Binary test: delete after a cancelled and a succeeded run keeps only the delivered branch
- [ ] B's facade rule exception lists the delete impact
