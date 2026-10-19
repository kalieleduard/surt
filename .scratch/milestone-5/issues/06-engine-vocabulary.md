# 06: The engine stops naming the feature workflow

Spec: [Milestone 5](../spec.md) (Engine vocabulary) · Plan: [H](../../../docs/architecture/H-artifact-storage.md), [A. Git Integration](../../../docs/architecture/A-git-integration.md)

**What to build:** the last feature words leave `surt-engine`.

**Blocked by:** 05

**Status:** open

- [ ] Working Copy at `.surt/subjects/<subject>/` (layout, working copy, driver, record, cleanup, purge, previous run, delete impact, CLI messages)
- [ ] Child branches `surt/item/<stem>/<key>`; integration merge message `Merge item <key> …` with trailer `Surt-Item:`
- [ ] Integration artifact key `items` (`integration.schema.json`, `drive/snapshot.rs`); the Template's Shipper instructions and delivery schema follow
- [ ] `StatusScope::Feature` → `StatusScope::Subject`; `base.schema.json`'s title is generic
- [ ] `rg -i "feature|task" crates/surt-engine/src` finds only tests, the Template's data or generic English
- [ ] H, A. Git Integration and D follow

## Comments
