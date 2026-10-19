# 06: The engine stops naming the feature workflow

Spec: [Milestone 5](../spec.md) (Engine vocabulary) · Plan: [H](../../../docs/architecture/H-artifact-storage.md), [A. Git Integration](../../../docs/architecture/A-git-integration.md)

**What to build:** the last feature words leave `surt-engine`.

**Blocked by:** 05

**Status:** resolved

- [x] Working Copy at `.surt/subjects/<subject>/` (layout, working copy, driver, record, cleanup, purge, previous run, delete impact, CLI messages)
- [x] Child branches `surt/item/<stem>/<key>`; integration merge message `Merge item <key> …` with trailer `Surt-Item:`
- [x] Integration artifact key `items` (`integration.schema.json`, `drive/snapshot.rs`); the Template's Shipper instructions and delivery schema follow
- [x] `StatusScope::Feature` → `StatusScope::Subject`; `base.schema.json`'s title is generic
- [x] `rg -i "feature|task" crates/surt-engine/src` finds only tests, the Template's data or generic English
- [x] H, A. Git Integration and D follow

## Comments

- Resolved on `m5/06-engine-vocabulary`. Engine: `Layout::working_copy` is `.surt/subjects/<subject>`; every user (driver, record, cleanup, purge, previous run, delete impact) goes through it, so only `layout.rs` changed. Child branches are `surt/item/<stem>/<key>` (`drive/worktrees.rs`); no other code matches refs by `surt/task/` (purge matches `refs/heads/surt/`, salvage refs are `refs/surt/salvage/<owner>`). The integration merge reads `Merge item <key> into <branch>` with trailers `Surt-Run`, `Surt-Item`, `Surt-Reviewed-Commit`; nothing parses the trailers back. `integration` lists `items` (engine schema, `docs/architecture/schemas/` copy, `drive/snapshot.rs`); the Template writes the shipper's copy from `INTEGRATION_SCHEMA`, so it follows (checked by `template/tests/schemas.rs`), and the Shipper's instructions name `items`. The delivery schema never named the key. `StatusScope::Feature` is `StatusScope::Subject`. `base.schema.json` was already generic (04).
- CLI: the awaiting-approval line (`driving.rs`), the recreated-file warning (`decide.rs`) and the unrecorded-edit refusals (`refusal/edits.rs`) print `.surt/subjects/<subject>/…`. CLI and engine tests follow; `follow.rs`, `interrupt.rs`, `claude.rs` and `cancel.rs` held none of these strings.
- `rg -n -i "feature|task" crates/surt-engine/src` outside tests now finds only `cfg(feature = "test-support")`, `tokio::task`/`std::task`, and `PermissionMode::DontAsk` (`dontAsk` matches `task` case-insensitively). Tests keep their `Feature` fixture, `task_tip`, `TWO_TASKS` and `tasks/A.md` item files: they script the feature-shaped synthetic workflow.
- Docs: H (Working Copy path, ignored `subjects/`), A. Git Integration (branch table and names, Working Copy path, merge message and trailer, `items` in the snapshot paragraph), D (child run ref), I-cleanup (Working Copy and branch names), J and K (`.surt/subjects/` in examples). B's rule exception for these leaks is removed. I-recovery named neither. Old runs are not migrated.
- For 07: `docs/smoke-claude-code.md` still uses `.surt/features/` and `surt/task/` (with the old commands); ADR 0007 keeps `Surt-Task` as the historical name; CONTEXT.md and the docs still call the child branch "Task Branch" (A now notes the `surt/item/` name is workflow-neutral).
