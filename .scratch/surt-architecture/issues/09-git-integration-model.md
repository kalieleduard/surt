# Git integration model

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Assignee: Kaliel
Blocked by: 01, 04

## Question

How does code flow from the Task branches to the branch delivered by the Shipper, guaranteeing that nothing unreviewed gets in?

Decide: name and base of the per-feature integration branch; which commit each Task branch starts from (the integration branch at start time?); who integrates an approved Task (deterministic native operation vs. agent) and when; what happens on conflict (a new Builder iteration, rebased? mandatory re-review?); how the review is bound to the exact SHA (and what invalidates the approval); who runs build/lint/test (commands declared in config and executed as a native step? do results become an artifact for the Reviewer?); the exact role of the Shipper (agent vs. native vs. hybrid) and what it delivers (final integrated branch, no merge into main); the layout of `~/.surt/worktrees/<project>/<feature>/<task>/` and project identification.

## Context

From [Agent contract](07-agent-contract.md): [F. Agent Contract](../../../docs/architecture/F-agent-contract.md) fixes the schemas of `commit`, `feedback` and `delivery`; this ticket defines Git evidence/SHA binding, check commands and the Shipper's final behavior. The per-Task `.md` files arrive through the generic item-files mechanism; mapping keys to physical paths must respect all three platforms, without using an unvalidated key directly as a file name.

Arose from [Git worktree/merge mechanics and partial states](04-git-worktree-merge-partial-states.md) — decide here: integration via `merge-tree` + `commit-tree` + `update-ref` (no worktree) vs. merging inside a worktree; minimum required Git version (≥2.38/2.40/2.43); the `git` CLI vs. libgit2/gix (research points to the CLI); never move, via `update-ref`, a branch checked out in another worktree; binding the review to the SHA and the `core.autocrlf` policy on Windows; configs Surt enforces or checks (`core.longpaths`, `core.lockfilePid`).

From [Process model and concurrency control](05-process-and-concurrency-model.md): Surt's Native Operations that touch the repo's shared state (worktree add/remove, branch creation, `update-ref`, `merge-tree`/`commit-tree`) run serialized under `.surt/runtime/locks/git.lock`, with retry and backoff for Git locks created by third parties; Builder commits in their own worktree stay outside this lock.

From [Workflow TOML schema](06-workflow-toml-schema.md): Operations are referenced in the TOML only by id (`run = "git.integrate"`); this ticket decides the **operation registry** (ids, arguments, declared Outcomes). Draft in use in section E: `git.create_integration_branch` (feature, `prepare`), `git.create_task_worktree` (task-cycle, `setup`), `checks.run` → `passed`/`failed` (placeholder for the `check` step, with a `check-report` artifact for the Reviewer) and `git.integrate` → `merged`/`conflict`. In the task-cycle, `integrate` runs inside the Loop after `review = approved`, and `conflict` triggers a new Iteration: decide what the Builder receives about the conflict. The Shipper reads only the `manifest` + the integration branch (the parent does not read the children's artifacts). Also here: restrictions on Item List `key`s so they can become branch/directory names (Windows reserved names, characters, case-insensitive collisions).

From [Artifact storage, freezing and revalidation](08-artifact-storage-freezing-revalidation.md): item files (`tasks/<key>.md`) mirror the Revision's relative paths in the Working Copy; the safe key→path encoding (Windows) is decided here. The Shipper does **not** commit the spec to the branch; whether the delivery text summarizes the spec is this ticket's decision. Builders read inputs from `.surt/runtime/executions/<id>/inputs/`, never from inside the worktree. See [H. Artifact Storage](../../../docs/architecture/H-artifact-storage.md).

## Answer

Resolved by human exchange over two rounds (plus an adjustment to "allow merge"). Complete section in [A. Git Integration](../../../docs/architecture/A-git-integration.md). The document is part of section A (architecture), not B: letter B in the brief is the Rust workspace (ticket 15), and the old mentions of "section B" in E/F/H were corrected. Trade-offs in [ADR 0006 — Merging into the Base is opt-in and fast-forward only](../../../docs/adr/0006-merge-into-base-is-opt-in-fast-forward.md) and [ADR 0007 — Tasks are integrated as merge commits of the reviewed SHA](../../../docs/adr/0007-integrate-reviewed-sha-as-merge-commit.md). The glossary gained **Base**, **Integration Branch**, **Task Branch**, **Conflict**, **Check** and **Initial Artifact**, and the Shipper was redefined as read-only.

Final decisions:

- **Integration:** native Operation `git.integrate`, without a worktree: `merge-tree --write-tree` → `commit-tree` (fixed dates = start of the Execution, deterministic SHA) → `update-ref --stdin` with CAS under `git.lock`. It is always a merge commit with parents `[tip, reviewed SHA]`, never squash/rebase/ff. Trailers `Surt-Run`/`Surt-Task`/`Surt-Reviewed-Commit`; the user's identity. Git ≥ 2.38, CLI only.
- **Names:** integration `surt/<subject>`, Task `surt/task/<subject>/<key>` (avoids the file-vs-directory ref conflict); runs N ≥ 2 of the same Subject use `<subject>.N`.
- **Base:** SHA of `HEAD` (or of `--base <ref>`) at `surt feature`, recorded as Initial Artifact `base` `{ref, sha}` (the engine generalizes `request` into the creating command's "initial artifacts"). `prepare` creates the integration branch exactly there; it warns if there are uncommitted changes.
- **Task start:** the integration tip at the Task's `setup`, recorded as `start`. Reading the start and the whole `integrate` are serialized per root run (a mutex in the Driver).
- **Worktrees:** `~/.surt/worktrees/<project-id>/<subject>[.N]/<key>/`, path recorded in SQLite. Project id = `<dir-slug≤24>-<6hex>`, generated when `surt.db` is created.
- **Keys/paths:** engine rules for every Item List: key `^[A-Za-z0-9][A-Za-z0-9_-]{0,31}$`, unique case-insensitively, no Windows reserved names; path components `[A-Za-z0-9._-]+` with equivalent rules; used verbatim, without encoding. The manifest schema was tightened.
- **Checks:** `[[checks]]` argv (no shell) with an optional `timeout` in `.surt/config.toml`, frozen in the root run (never read from the worktree). `checks.run` runs all of them, in order, on the Builder's SHA → `check-report` (status, exit, duration, last 200 lines). `failed` skips review and integrate (`when` on `review`), and the Builder receives `check-report?`.
- **Commit evidence:** it exists; it is `HEAD` and the tip of the Task Branch; it descends from the start (and from the tip, if there was a conflict) with a new commit; the worktree is clean with no operation in progress. Failure = invalid delivery.
- **Binding:** `check-report.commit` = `feedback.reviewed_commit` = `commit.sha`; `integrate` merges the approved SHA, not the branch. Only a new SHA invalidates the approval.
- **Clean vs. broken merge:** if the tip has moved since the start, the merge is re-checked in the Task's worktree (`checkout --detach M`) before the `update-ref`. Invariant: every tip of the Integration Branch has passed the Checks.
- **Conflict:** `git.integrate` → `conflict` with a `conflict` artifact (`kind = merge|checks`, tip, reviewed SHA, paths or checks). The Builder merges (not rebases) the tip and goes through full check and review again. The Context Assembler labels each Input with its originating Iteration/step and says why the Iteration started (F updated).
- **Ownership of the Integration Branch:** refuses if it is checked out in any worktree; stops (without adopting) if the tip diverges from the recorded one; always shows the fix command.
- **Shipper:** new `snapshot` step (`git.snapshot_integration` → artifact `integration` {branch, base, tip, tasks}). The Shipper is read-only, reads `manifest` + `integration` and delivers `delivery` = `{title, summary}`; branch/SHA come from `integration`. Nothing is pushed; the spec is not committed.
- **Merge into the Base (opt-in):** `merge` step (`git.merge_into_base`) after `ship`, controlled by `[git] merge_into_base = "never" | "ff-only"` (default `never`, frozen). Fast-forward only: skips if the Base has moved, if it is not a branch or if the worktree where it is checked out is not clean; in a clean worktree it uses `merge --ff-only` inside it. Outcomes `merged` · `skipped`. The brief's restriction was relaxed to "unless opted in".
- **Git config:** `--no-optional-locks` on reads; `-c core.longpaths=true` (Windows) and `-c core.lockfilePid=true` (Git ≥ 2.56) on Surt's commands; `core.longpaths` injected into the agent via `GIT_CONFIG_*`; never writes `.git/config`; does not enforce `autocrlf`. Preflight: version, non-bare repo, identity.

Documentation validation: every TOML block in A/E/F parses (`tomllib`); the new JSON schemas (`base`, `check-report`, `conflict`, `integration`, `delivery`) parse. Context notes added to [Builder ↔ Reviewer loop state machine](10-builder-reviewer-loop.md), [SQLite schema](11-sqlite-schema.md), [Recovery and reconciliation strategy](12-recovery-and-reconciliation.md), [Cleanup and lifecycle safety](13-cleanup-delete-and-logs.md), [CLI surface and UX](14-cli-surface-and-ux.md), [Rust workspace structure and core traits](15-rust-workspace-and-traits.md) and [Test strategy](16-test-strategy.md). No new ticket: what was left fits in those tickets.
