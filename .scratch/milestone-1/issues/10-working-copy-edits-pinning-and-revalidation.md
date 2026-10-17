# 10: Working Copy edits, pinning and revalidation

Spec: [Milestone 1](../spec.md) (stories 22–23, 30–34, 38–39) · Slice: S3 Engine + Fake · Plan: [H](../../../docs/architecture/H-artifact-storage.md), [ADR 0004](../../../docs/adr/0004-item-list-files-share-approved-revision.md), [ADR 0005](../../../docs/adr/0005-edits-to-pinned-revisions-are-refused.md)

**What to build:** the developer can edit the spec, the manifest and `tasks/*.md` in their editor and Surt versions the edit as a human Revision, never losing it and never letting unapproved or changed content flow downstream. Deleting a file is how an unrecorded edit is undone while `surt restore` doesn't exist.

**Blocked by:** 09 (Spec and plan phases through the binary)

**Status:** ready-for-agent

- [ ] A Working Copy exists only for artifacts with an Approval Gate; edits are detected by full SHA-256 against the baseline, never mtime
- [ ] An edit becomes a human-origin Revision when `approve`, `reject` or a driving command runs; the Approval targets exactly that Revision
- [ ] A deleted Working Copy file is recreated from the latest Revision by the next command that acts
- [ ] Editing an approved but not yet consumed Revision records an unapproved Revision and every driving command refuses until it is approved
- [ ] Editing a Pinned Revision is refused, not recorded: the Driver lets running work finish, launches nothing new and exits `3` (`Halted`); refusals say `fix: rm <file>, then rerun`
- [ ] An agent Revision arriving over an unrecorded edit first saves the edit as its own Revision and warns
- [ ] The manifest JSON and every Task file are one Revision; an edit is validated like an agent delivery (schema, unique keys, valid `needs`, no cycles, every referenced file present, no unreferenced file, safe paths) and an invalid edit creates no Revision and refuses
- [ ] `Project` tests for each case above, plus a binary test of the `fix: rm` refusal
