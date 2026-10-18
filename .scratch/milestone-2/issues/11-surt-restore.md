# 11: `surt restore`

Spec: [Milestone 2](../spec.md) (stories 40–48) · Slice: R5 Restore · Plan: [H. Working Copy](../../../docs/architecture/H-artifact-storage.md#working-copy), [H. Revalidation](../../../docs/architecture/H-artifact-storage.md#revalidation), [I. Unrecorded Working Copy edits](../../../docs/architecture/I-cleanup.md#unrecorded-working-copy-edits)

**What to build:** a developer can back out of an unrecorded Working Copy edit with one command. `surt restore <feature> <artifact>` shows what will be lost, asks `[y/N]`, and rewrites the Working Copy to exactly the latest recorded Revision without creating a Revision. The pinned-edit and revalidation refusals now suggest this command instead of `rm`.

**Blocked by:** 02 (Events carry their details)

**Status:** ready-for-agent

- [x] A restore impact read companion lists, per file: changed with added/removed line counts (counted in-house, no new dependency), new, or missing
- [x] The CLI shows the impact and asks `[y/N]`; `--yes` skips it; without a TTY and without `--yes` it refuses with exit `3`
- [x] Nothing differs: says so and exits `0`
- [x] Acts only on the Subject's active run, under that root run's lock taken with `try_lock`; a live Driver refuses, naming its pid
- [x] Settles a pending Working Copy write for the artifact first, with reconciliation's code, so a half-published agent Revision is never taken for an edit
- [x] Afterwards the Working Copy equals the Revision's file set (extra files removed, missing ones recreated, changed ones rewritten) and the baseline equals the file hashes; no Revision, Approval or pin changes; one event records it
- [x] An artifact without a Working Copy refuses and lists the restorable artifacts
- [x] A terminal run refuses with I's `fix: move or delete the file` wording
- [x] The H refusals say `undo: surt restore <feature> <artifact>` instead of `fix: rm <file>, then rerun`
- [x] `Project` tests: no Revision created, pending write settled, live-Driver and terminal-run refusals
- [x] Binary test: a pinned-edit refusal is undone with `surt restore --yes` and the run continues
- [x] H and I record the new wording and the terminal-run rule; J records the active-run-only rule; B's facade rule exception lists the restore impact
