# H. Artifact Storage, Freezing and Revalidation

Where Artifact Revisions live, how humans read and edit them, and what happens when an approved artifact changes. Vocabulary (**Artifact**, **Artifact Revision**, **Working Copy**, **Pinned Revision**, **Approval**) is defined in [`CONTEXT.md`](../../CONTEXT.md); invariants are in [C. Domain Model](C-domain-model.md). Decision ticket: [Artifact storage, freezing and revalidation](../../.scratch/surt-architecture/issues/08-artifact-storage-freezing-revalidation.md).

## Revision bytes live in SQLite

Every file of every Revision is stored as a content-addressed blob inside `.surt/surt.db`:

```sql
-- Indicative; the physical schema is D-database.md (tables blob, revision_file).
CREATE TABLE blob (
  sha256 TEXT PRIMARY KEY,          -- lowercase hex of the bytes
  bytes  BLOB NOT NULL
) STRICT;

CREATE TABLE revision_file (
  revision_id TEXT    NOT NULL,
  rel_path    TEXT    NOT NULL,     -- portable relative path, '/' separator
  sha256      TEXT    NOT NULL REFERENCES blob(sha256),
  PRIMARY KEY (revision_id, rel_path)
) STRICT;
```

- Publishing a Revision is **one transaction**: blobs, Revision row, file rows and the Step Run transition commit together or not at all. There is no file store to reconcile after a crash, and no Revision pointing at missing bytes.
- Identical bytes are stored once. A Revision's identity hash is the SHA-256 over its sorted `(rel_path, sha256)` pairs, so an Item List Revision's identity covers its JSON and every referenced file ([ADR 0004](../adr/0004-item-list-files-share-approved-revision.md)).
- Revisions are never updated or deleted while their Workflow Run exists. Earlier Revisions (a rejected spec, a superseded manifest) stay readable with `surt show <feature> <artifact> --rev N`.
- Backing up a project's Surt state means `VACUUM INTO '<backup-path>'` or SQLite's backup API, which read a consistent snapshot even while a Driver writes. A plain file copy of `surt.db` is not a backup: in WAL mode, transactions committed by a Driver that crashed can still live only in `surt.db-wal`.

## What freezing means at approval

Nothing is copied when a human approves. An Approval records the `revision_id` and the Revision's identity hash, plus who and when. The Revision is already immutable, so pointing at it is enough.

`surt approve` first scans the Working Copy (below), then approves the **latest** Revision and says which one:

```
$ surt approve 20261016-add-login
✔ approved spec rev 3 (human edit) sha256:3f9a2c…
```

`--rev N` is an optional guard: the command refuses if the latest Revision is not `N`. Scripts, and anyone who wants to approve exactly what they read, use it.

## Working Copy

Humans read and edit artifacts through a **Working Copy** under `.surt/subjects/<subject>/`. It only exists for artifacts that pass through an Approval Gate in the run's frozen Workflow Definition. The engine derives the list from the `kind = "approval"` steps; it does not know what a spec or a manifest is. For the Template's `feature` workflow:

```
.surt/subjects/20261016-add-login/
├── spec.md          ← artifact of gate approve-spec
├── manifest.json    ← artifact of gate approve-tasks
└── tasks/
    ├── A.md         ← item files of the manifest Revision
    └── B.md
```

File paths mirror the Revision's relative paths verbatim. Keys and path components follow the portable naming rules of [A. Git Integration](A-git-integration.md#item-keys-and-item-file-paths), so no encoding is needed.

Every other artifact (`request`, `feedback`, `*-rejection-reason`, Shipper outputs) has no Working Copy. Humans read it with `surt show <subject> <artifact> [--rev N]`. This keeps a human from editing, for example, the Reviewer's `feedback` in the middle of a Loop.

Surt rewrites a Working Copy file whenever it publishes a newer Revision of that artifact. For each file it writes, it records in SQLite the SHA-256 it materialized: the file's **baseline**. The new baseline is first recorded as pending in the publication transaction and promoted once the file is written, so a crash between the two is never mistaken for a human edit ([I. Recovery](I-recovery.md#working-copy-files)).

## Edit detection

- **When:** every command that acts on a Subject scans that Subject's Working Copy first and records what it finds: `approve`, `reject`, `continue`. Read commands (`status`, `inspect`, `show`, `logs`) compare and show an edit but never record it ([J. Reading never writes](J-cli.md#reading-never-writes)). A Driver also scans before it launches each Execution, because that is when inputs are resolved, and before it rewrites a Working Copy file.
- **How:** a full SHA-256 of every file compared with its baseline. Modification times are never used, because their granularity and behaviour differ across APFS, NTFS and ext4, and the files are small.
- **Edit:** if any file differs from its baseline, the artifact's new content becomes a **human-origin Revision**, and its hashes become the new baseline:
  ```
  $ surt approve 20261016-add-login
  ✔ approved spec rev 2 (human edit) sha256:9b0e41…
  ```
- **Deleted file:** a missing Working Copy file is not an edit. A command that acts warns and recreates it from the latest Revision; a read command only shows it.
- **Pinned artifact:** an edit to an artifact whose approved Revision is pinned is never recorded (see [Revalidation](#revalidation)).

### Editing an Item List

A scan of an Item List artifact assembles the whole set, the JSON index and every referenced file, and validates it with **the same rules as an agent delivery** ([F. Agent Contract](F-agent-contract.md)): schema, unique keys, valid `needs`, every referenced file present, non-empty UTF-8 Markdown, no unreferenced file, safe relative paths. An invalid edit creates **no Revision**, and the command refuses:

```
$ surt approve 20261016-add-login
✖ manifest edit is invalid, no revision recorded:
  tasks/C.md is not referenced by any item
  undo: surt restore 20261016-add-login manifest
```

`surt restore <subject> <artifact>` rewrites the Working Copy from the latest recorded Revision and discards the unrecorded edit. It creates no Revision, no Approval and no pin, and records one `working_copy.restored` event:

- the Working Copy ends up equal to the Revision's file set: changed files are rewritten, missing ones recreated, files the Revision does not have are removed; the baseline already holds those hashes;
- it acts only on the Subject's **active** run, under that root run's lock taken with `try_lock`; a live Driver refuses with its pid ([J](J-cli.md#surt-restore));
- before comparing, it settles a pending Working Copy write with the same code reconciliation uses ([I. Recovery](I-recovery.md#working-copy-files)), so a half-published agent Revision is never taken for an edit;
- only artifacts with a Working Copy (behind an Approval Gate) can be restored; another name refuses and lists those that can;
- on a terminal run it refuses and keeps the file ([I. Cleanup](I-cleanup.md#unrecorded-working-copy-edits)).

Both refusals of an edit, the invalid edit above and the pinned edit below, name this command as their way out (`undo: surt restore <subject> <artifact>`), never `rm <file>`.

## Revalidation

Steps read artifacts by name, at the latest Revision ([ADR 0003](../adr/0003-inputs-resolved-by-artifact-name.md)). This section is therefore the only protection against unapproved content flowing downstream. There are two cases.

**Approved, not yet consumed.** An edit is recorded as Revision N+1 with no Approval. Every driving command refuses until a human approves it:

```
$ surt continue 20261016-add-login
✖ spec changed after approval (rev 2 not approved)
  review it, then: surt approve 20261016-add-login
```

`surt approve` may record an Approval on the latest Revision of an artifact whose gate has already passed, for exactly this purpose.

**Pinned.** A Revision becomes a **Pinned Revision** the first time any Execution reads it as an Input after it was approved. Expanding a Per-Item Step pins its Item List Revision, because its children read their items from it. From then on, for the rest of the Workflow Run:

- an edit to that artifact's Working Copy is **refused, not recorded** ([ADR 0005](../adr/0005-edits-to-pinned-revisions-are-refused.md));
- `surt approve` refuses for that artifact;
- a Driver that detects such an edit launches nothing new, lets running Executions finish so no work is lost, and exits;
- the way out is `surt restore` (undo the edit) or `surt cancel` followed by a new run.

```
$ surt continue 20261016-add-login
…
✖ tasks/C.md changed, but manifest rev 1 is pinned (implementation already started)
  running work finished; nothing new was launched
  undo:     surt restore 20261016-add-login manifest
  replan:   surt cancel 20261016-add-login, then surt continue 20261016-add-login --new-run
```

Both cases on one timeline:

```
spec rev1 ─approve─▶ edit ─▶ rev2 (unapproved) ─approve─▶ plan reads rev2 ✔
spec rev1 ─approve─▶ plan reads rev1 (PINNED) ─▶ edit ─▶ ✖ refused: restore or cancel
```

A rejected Revision is never pinned. On a later Iteration, the Spec Agent reads its rejected `spec` together with `spec-rejection-reason`.

### Agent output while the Working Copy has an unrecorded edit

Before rewriting a Working Copy file for a new agent Revision, the Driver scans. An unrecorded edit is recorded first as a human-origin Revision. The agent's delivery then becomes the newer Revision and overwrites the file. Nothing is lost:

```
⚠ your edit to spec.md was saved as rev 2 and superseded by the agent's rev 3
  see it:  surt show 20261016-add-login spec --rev 2
```

The agent did not read that edit, because its Inputs were fixed when its Execution started.

## Execution directories

Agents never read the Working Copy. Each Execution gets its own directory, created again for every Attempt from the blobs:

```
.surt/runtime/executions/<execution-id>/
├── inputs/            ← read-only bytes of the Input Revisions fixed at start
│   ├── spec.md
│   ├── item.json
│   ├── item/task.md
│   └── feedback.json  (second Iteration onwards)
└── delivery/          ← where the agent writes its outputs (section F)
```

The Context Assembler points the agent at `inputs/`, and the runtime adds `Read` on that absolute root to the permission rules ([F. Agent Contract](F-agent-contract.md)). A Builder working in `~/.surt/worktrees/…` reads its inputs from this directory. They are not copied into the worktree, where they would show in `git status` and could get committed. Materializing inputs creates no Revision.

## Subject naming

`surt run` derives the Subject name `YYYYMMDD-<slug>` from `--name`, else from the input the workflow's `[subject] from` names (the feature Template's `request`), else from the workflow name ([E](E-workflow-schema.md#run-inputs-inputs-and-subject)):

1. `YYYYMMDD` is the **local** date. If the local offset cannot be determined, Surt uses UTC without warning.
2. The slug is the text transliterated to ASCII, lowercased, reduced to `[a-z0-9-]`, with runs of hyphens collapsed and no leading or trailing hyphen. It is truncated to 40 characters at a word boundary. `"Add Login with Google / OAuth"` → `20261016-add-login-with-google-oauth`.
3. Windows reserved names (`CON`, `NUL`, `COM1`…) cannot occur, because the name always starts with digits. Lowercase removes case-insensitive collisions on APFS and NTFS. A trailing dot or space cannot survive the slug.
4. If a Subject with that name already exists, Surt appends `-2`, `-3`… Exception: when a Subject with the same slug (ignoring date and suffix) has an active run, `surt run` refuses and points at resuming it, unless `--new` is given ([J](J-cli.md#surt-run-create-a-run-and-drive-it)):
   ```
   $ surt run feature --input request="add login" --new
   ✔ created 20261016-add-login-2   workflow: feature   base: main @ a11c9d0
     20261016-add-login already exists
   ```
5. If the slug is empty (a request only in kanji or emoji), Surt refuses:
   ```
   ✖ cannot derive a name from the subject input; pass --name <slug>
   ```
   `--name` may be given in any case and goes through the same rules, except the date prefix, which is always added.

## What is versioned in the user's Git

Surt writes `.surt/.gitignore` so that only the project's configuration and definitions are versioned:

```
.surt/
├── .gitignore         ✔ versioned (generated by Surt)
├── config.toml        ✔ versioned (project configuration)
├── workflows/         ✔ versioned (Workflow Definitions, section E)
├── agents/            ✔ versioned (Agents, section F)
├── subjects/          ✖ ignored: Working Copies; the truth is in SQLite
├── logs/              ✖ ignored: agent and Check logs, one directory per Subject
├── surt.db            ✖ ignored (with -wal and -shm)
└── runtime/           ✖ ignored: locks, slots, Execution directories
```

```gitignore
# generated by surt
*
!.gitignore
!config.toml
!workflows/
!workflows/**
!agents/
!agents/**
```

`workflows/` and `agents/` hold the complete definitions a new run freezes ([E](E-workflow-schema.md#where-definitions-live), [F](F-agent-contract.md#definitions-in-the-project)); `surt init --template feature` writes the feature Template there and never overwrites a file. They are reviewed like any other code: pulling a branch can change what the next run does.

Versioning `subjects/` would create a second source of truth. Task branches would carry stale copies of the spec, and a `git checkout` of another branch would change the Working Copy, which Surt would see as a human edit.

## After a run ends

- Revisions, blobs and Approvals stay in SQLite for as long as the Workflow Run exists. Removing old runs (purge) is section I.
- Cleanup of a terminal run removes its Working Copy `.surt/subjects/<subject>/` and its `.surt/runtime/executions/<id>/` directories. Both are views that can be regenerated: `surt show <subject> <artifact>` reads any Revision. A Working Copy file with an unrecorded edit is kept, and `--new-run` refuses while it is there ([I. Cleanup](I-cleanup.md#unrecorded-working-copy-edits)).
- Agent logs live in `.surt/logs/<subject>/`, outside `runtime/`, and survive cleanup ([I. Cleanup](I-cleanup.md#logs)).
- Nothing is committed into the user's branches. The Shipper does not commit the spec; its delivery text summarizes the spec's intent ([A. Git Integration](A-git-integration.md#snapshot-shipper-and-delivery)).

## Decided elsewhere

- Physical tables for blobs, Revisions, baselines and Approvals: [D. Database Schema](D-database.md#artifacts-revisions-and-approvals).
- Item key and path naming rules, worktree layout: [A. Git Integration](A-git-integration.md) ([Git integration model](../../.scratch/surt-architecture/issues/09-git-integration-model.md)).
- What cleanup removes, when, and purge of old runs: [I. Cleanup, Delete and Logs](I-cleanup.md).
- Exact output and exit codes of `show`, `restore`, `approve --rev` and the refusals: [J. CLI Surface](J-cli.md).
- Where logs and streams live and when they are removed: [I. Cleanup](I-cleanup.md#logs).

Decision record: [ADR 0005 — Edits to a pinned Revision are refused, not recorded](../adr/0005-edits-to-pinned-revisions-are-refused.md).
