# Artifact storage, freezing and revalidation

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Assignee: Kaliel
Blocked by: 01

## Question

Where do artifacts live, how are they versioned and frozen, and what happens when they change after approval?

Decide: does `.surt/features/<feature>/` go into the user's Git or is it ignored?; how do Builders in worktrees outside the repo see the spec/tasks (a frozen copy in `runtime/runs/<id>/inputs/`? content-addressed by SHA-256?); what "freezing" means at approval; when a new artifact revision is born (human edit, regeneration); the revalidation policy when an approved doc is edited (block, require re-approval, ignore — by whom and when it is detected); how `surt plan` regenerates spec/tasks and what happens to earlier revisions; what is left of the spec after cleanup (only hashes in SQLite? something in the Shipper's commit?); feature naming and collision resolution.

## Context

Arose from [Cross-platform primitives](18-cross-platform-primitives.md): normalizing `description` must avoid Windows reserved names (CON, NUL, COM1…), forbidden characters and a trailing dot/space, and case-insensitive collisions; the local date can fail (`IndeterminateOffset`) — define a fallback.

From [Domain glossary](01-domain-glossary.md): an Approval points to a specific **Artifact Revision** and is never inherited by a new Revision; every Revision is immutable and has a single origin (an Execution or a human edit); Executions read Revisions fixed when they start. The revalidation policy decides what to do about this, not whether the approval "migrates".

From [Workflow TOML schema](06-workflow-toml-schema.md): steps read artifacts **by name, at the latest Revision** when the Execution starts (not "the Revision the gate approved"). That is why this revalidation policy is the **only** protection for "nothing unapproved flows downstream". Behavior already chosen by the user: a driving command refuses when the latest Revision of an artifact that went through a gate is not approved:
```
$ surt plan 20261016-add-login
✖ spec changed after approval (rev 2 not approved)
  review it, then: surt approve 20261016-add-login
```
Still missing here: how the edit is detected (a hash at the start of each command?), and the same for edits during `surt start`.

From [Agent contract](07-agent-contract.md): an Item List Revision includes the JSON and every file referenced by `files`; in the feature workflow, it includes one `.md` written by the Task Agent for each Task. Approval covers the whole set; editing any `.md` requires a new Revision and re-approval. This ticket decides persistence, edit detection, local views and revalidation with children already started, preserving the snapshot fixed at expansion. Canonical contract: [F. Agent Contract](../../../docs/architecture/F-agent-contract.md#one-markdown-document-per-task).

## Grilling

### Round 1 — storage, editing, Git, inputs, name

The human answered `I agree with all of them` (Q1–Q5): content-addressed blobs in SQLite with atomic publication and Approval = `revision_id` + hash; an editable Working Copy in `.surt/features/<subject>/` with detection by Surt; `.surt/.gitignore` versions only `config.toml` and `agents/`; a per-Execution directory `runtime/executions/<id>/{inputs,delivery}/` materialized from the blobs; name `YYYYMMDD-<slug>` with the five rules (local date with UTC fallback, ASCII slug ≤40, no reserved names thanks to the numeric prefix, `-2` suffix, `--name` when the slug comes out empty).

### Round 2 — detection, revalidation, conflicts, end of the run

The human answered `I approve everything` (Q6–Q11): a SHA-256 scan (never mtime) at the start of every feature command and before each Execution, a deleted file is recreated; an edited Item List is validated like an agent delivery, an invalid edit does not become a Revision (`surt restore`); an approved and consumed Revision is pinned, a later edit is refused, the Driver drains and exits; an edit not captured before an agent's delivery becomes a human Revision and is superseded; `approve` scans, approves the latest, `--rev` as a guard; SQLite keeps everything until purge, the Working Copy and Execution directories go away at cleanup, nothing is committed.

### Round 3 — Working Copy scope, term, ADR

The human answered `I agree with all of them` (Q12–Q14): Working Copy only for artifacts with an Approval Gate (derived from the frozen definition); the term **Pinned Revision** in the glossary; ADR 0005 only.

## Answer

Resolved by human exchange over the three rounds above. Complete section in [H. Artifact Storage, Freezing and Revalidation](../../../docs/architecture/H-artifact-storage.md); trade-off in [ADR 0005 — Edits to a pinned Revision are refused, not recorded](../../../docs/adr/0005-edits-to-pinned-revisions-are-refused.md). The glossary gained **Working Copy** and **Pinned Revision**; sections A, C (invariant 7), E, F and ADR 0003 point to H.

Final decisions:

- **Storage:** the bytes of every Revision as SHA-256 blobs in `surt.db`; publishing = one transaction; Revision identity = hash of the `(rel_path, sha256)` pairs; Revisions are never deleted while the run exists.
- **Freezing at approval:** nothing is copied; the Approval points to `revision_id` + hash. `surt approve` scans, approves the latest, prints rev + hash; `--rev N` is a guard.
- **Working Copy:** only gated artifacts (`spec`, `manifest` + `tasks/*.md` in `feature`), derived from the `kind = "approval"` steps; everything else through `surt show`. Agents never read the Working Copy.
- **Detection:** SHA-256 against a baseline on every feature command and before each Execution/rewrite; a deleted file is recreated; an edited Item List goes through delivery validation, an invalid one does not become a Revision (`surt restore`).
- **Revalidation:** approved and not consumed → the edit becomes rev N+1 and driving commands refuse until `approve`. **Pinned** (approved and read as an Input; the Per-Item expansion pins the manifest) → the edit is refused, `approve` refuses, the Driver launches nothing new, lets running work finish and exits; the ways out are `restore` or `cancel`.
- **Conflict with an agent delivery:** an uncaptured edit becomes a human Revision first; the agent's becomes the latest, overwrites it and warns.
- **Execution Inputs:** `.surt/runtime/executions/<id>/{inputs,delivery}/`, recreated per Attempt from the blobs; `Read` on the absolute path; nothing copied into the worktree.
- **Git:** generated `.surt/.gitignore` (`*`, `!.gitignore`, `!config.toml`, `!agents/**`) — checked with `git add -A` in a test repo.
- **Name:** `YYYYMMDD-<slug>`, local date with UTC fallback, ASCII slug `[a-z0-9-]` ≤40 on a word boundary, `-N` suffix on collision, `--name` (same rules, date prefix always present).
- **End of the run:** SQLite keeps Revisions/Approvals until purge (cleanup ticket); cleanup removes the Working Copy and Execution directories; nothing committed, the Shipper does not commit the spec.

Documentation validation: SQL sketch run in `sqlite3`; `.gitignore` rules checked in a throwaway repo. Context notes added to [Git integration model](09-git-integration-model.md), [SQLite schema](11-sqlite-schema.md), [Cleanup and lifecycle safety](13-cleanup-delete-and-logs.md), [CLI surface and UX](14-cli-surface-and-ux.md) and [Log, retention and redaction policy](13-cleanup-delete-and-logs.md) (later absorbed into 13). No new ticket: what was left (tables, path encoding, purge, CLI outputs) already fits in those tickets.
