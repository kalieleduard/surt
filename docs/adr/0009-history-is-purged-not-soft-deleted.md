# History is purged, not soft-deleted

No table has a `deleted_at` column. Removing a Subject's history is a purge: one transaction deletes the `subject` row, foreign keys cascade through its runs, Step Runs, Executions, Revisions, Approvals and events, and unreferenced snapshots and blobs are then deleted. The `event` table is append-only by trigger and leaves only with its Subject.

## Considered Options

- **Soft delete with `deleted_at` on every table (the brief's suggestion):** every read needs `WHERE deleted_at IS NULL`, every unique index has to include the column, a deleted Subject's name stays taken, and its bytes never leave `surt.db`. "Append-only" events would need an exception for marking them deleted.
- **Soft delete only on `subject`:** hides a Subject but keeps everything else, which still leaves the bytes and still needs filters in every listing.
- **Hard purge by cascade (chosen):** one statement, no filters anywhere, the name is free again, and space is reclaimed. Composition foreign keys cascade; cross-references use `NO ACTION`, which is checked after the cascade finishes, so a single `DELETE` succeeds.

## Consequences

- A purge cannot be undone. Section I decides when one is offered and how it is confirmed.
- Hiding a Subject without deleting it, if ever wanted, is an additive column on `subject`.
- Backups use `VACUUM INTO` or SQLite's backup API, never a plain copy of `surt.db`, which in WAL mode can miss committed transactions still in `surt.db-wal` ([H](../architecture/H-artifact-storage.md)).
