# 05: Fast-forward when the Base branch is checked out

Spec: [Milestone 4](../spec.md) (stories 42–46, 50–52, 60) · Plan: [A. Merging into the Base](../../../docs/architecture/A-git-integration.md#merging-into-the-base-opt-in), [I. An interrupted fast-forward into the Base](../../../docs/architecture/I-recovery.md#an-interrupted-fast-forward-into-the-base)

**What to build:** most developers have `main` checked out in their own worktree. When that worktree is clean, Surt runs the fast-forward inside it so files and branch move together. When it is not clean, Surt leaves it alone and says so. If a second Ctrl-C interrupts the checkout halfway, the next command stops with the exact commands to finish or undo it, and picks up whatever the developer did.

```
⚠ not merged: main is checked out in /Users/you/surt with changes; merge surt/20261017-add-login yourself

⛔ merge into main was interrupted; /Users/you/surt may be partially updated
   finish it:  git -C /Users/you/surt merge --ff-only 4d2e9a1
   or undo it: git -C /Users/you/surt reset --hard a11c9d0   (discards changes)
   then: surt start 20261017-add-login
```

**Blocked by:** 04 (Fast-forward into a Base branch that is not checked out)

**Status:** ready-for-agent

- [ ] The worktree holding `refs/heads/<base.ref>` is found with `git worktree list --porcelain -z`
- [ ] "Clean" means no staged or unstaged changes to tracked files and no merge, cherry-pick, revert or rebase in progress; untracked files do not count
- [ ] A clean worktree gets `git -C <worktree> merge --ff-only <tip>` → `merged`, with `worktree_path` in the `ref_move` row; a worktree that is not clean → `skipped` (`worktree_not_clean`)
- [ ] When `git merge --ff-only` refuses because it would overwrite an untracked file, the Outcome is `skipped` (`worktree_not_clean`)
- [ ] With `worktree_path` set, the ref at `old_sha` and that worktree not clean, the Step Run becomes `blocked` (`not_retryable`, `merge_into_base_interrupted`), exit `4`, with I's finish/undo lines; after "finish it" the next drive records `merged`, after "undo it" it fast-forwards again, after nothing it blocks again
- [ ] A `git merge --ff-only` failure (a Windows file in use) is a technical failure whose next Attempt follows the rules above
- [ ] The repository's `post-merge` hook runs as Git runs it; its exit status does not change the Outcome
- [ ] `Project` tests with real Git: clean checked-out worktree (files and ref both at the tip), tracked change, in-progress merge marker, untracked file left alone, untracked file that would be overwritten; the interrupted case with its finish and undo paths
- [ ] Binary test: `merge_into_base_interrupted` exits `4` with the finish/undo lines
- [ ] A records the definition of clean and the hook behaviour; I links the reason values
