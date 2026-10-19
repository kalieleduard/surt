# 04: Fast-forward into a Base branch that is not checked out

Spec: [Milestone 4](../spec.md) (stories 31, 37, 40, 41, 47–49, 53, 54, 57–59) · Plan: [A. Merging into the Base](../../../docs/architecture/A-git-integration.md#merging-into-the-base-opt-in), [I. Ref moves](../../../docs/architecture/I-recovery.md#ref-moves), [ADR 0006](../../../docs/adr/0006-merge-into-base-is-opt-in-fast-forward.md)

**What to build:** with `[git] merge_into_base = "ff-only"` in `.surt/config.toml`, a Feature whose Base branch did not move lands on it with no manual step: after the Shipper, Surt moves the branch to the delivered tip with a compare-and-swap. When the Base moved or was not a branch, Surt says why and leaves the merge to the developer; the run still succeeds. Surt never creates a merge commit on the Base branch and never pushes.

```
✔ shipper    delivered — surt/20261017-add-login @ 4e1b2c0
✔ merged into main (fast-forward a11c9d0 → 4e1b2c0)
✔ run        succeeded

⚠ not merged: main moved since base (a11c9d0 → b22e310); merge surt/20261017-add-login yourself
```

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] `merge_into_base = "ff-only"` parses; the `MergeIntoBaseUnsupported` refusal and its tests are removed; the `surt init` template drops "not supported yet"
- [ ] `feature.toml` gains `[steps.merge]` (`operation`, `git.merge_into_base`, `needs = ["ship"]`); runs frozen before M4 have no merge step and finish as before
- [ ] `git.merge_into_base` follows A's table in order: `never` → `skipped` (`disabled`); `base.ref` null → `skipped` (`no_base_branch`); branch not at `base.sha` → `skipped` (`base_moved`); branch not checked out → `update-ref` with `base.sha` as old value under `git.lock`, reflog message `surt: fast-forward <base.ref> to surt/<subject>` → `merged`, a failed compare-and-swap → `skipped` (`base_moved`)
- [ ] The `ref_move` Intention is written before the ref moves and deleted with the Outcome; recovery settles it by reading Git (ref at `new_sha` → `merged`, at `old_sha` → run again, anything else → `skipped`, `base_moved`)
- [ ] `step.done` for this step carries `reason` on `skipped`, `from`/`to` on `merged`, and `found` on `base_moved`; D's field table lists them
- [ ] The CLI prints `✔ merged into …` or `⚠ not merged: …` naming the reason and `merge surt/<subject> yourself`; `disabled` prints nothing; every skip keeps exit `0`; glyphs go through the symbol table
- [ ] The step runs before the run is recorded `succeeded` and before cleanup
- [ ] `Project` tests with real Git: `never`, detached Base, moved Base, branch not checked out; the run ends `succeeded` in each, with the recorded reason or SHAs
- [ ] Fault points: a crash at the `Intention` checkpoint and a crash after the ref moved but before the Outcome was recorded both settle on the next drive
- [ ] Binary tests: the merged and not-merged lines and exit `0` (replacing the test that expected `ff-only` to refuse)
- [ ] A records the reason values, the reflog message and the output lines; K's `merge_into_base` row says it is implemented
