# Tasks are integrated as merge commits of the reviewed SHA

`git.integrate` always records an approved Task as a merge commit on the Integration Branch whose parents are the previous tip and the exact SHA the Reviewer approved, even when a fast-forward would be possible. It builds that commit without a worktree (`merge-tree --write-tree` → `commit-tree` with fixed dates → `update-ref` with compare-and-swap), so a new Attempt recomputes the same SHA. A reader may wonder why the delivered history is not one tidy commit per Task. The reason is proof: `git merge-base --is-ancestor <reviewed> surt/<subject>` shows, with Git alone, that what was integrated is what was reviewed. If the tip moved since the Task started, the merge result passes the Checks again before the ref moves.

## Considered Options

- **Squash each Task:** one commit per Task, but its SHA is not the reviewed SHA, so "nothing unreviewed got in" can no longer be checked from history.
- **Rebase each Task onto the tip:** linear history, but the reviewed commits disappear and the rebased ones were never reviewed or checked.
- **Fast-forward when possible:** saves a commit, but the Integration Branch would have two shapes depending on timing, and the per-Task trailers (`Surt-Task`, `Surt-Reviewed-Commit`) would have nowhere to live.
- **Merge inside a dedicated integration worktree:** works on older Git, but adds a checkout per Feature with more partial states (`MERGE_HEAD`, `index.lock`) to reconcile.

## Consequences

- Git 2.38 or later is required.
- The delivered branch shows the Builders' commits plus one merge per Task. A user who wants one commit squashes when merging into `main`; Surt never rewrites the history it proved.
- A conflict produces no commit. The Builder merges the tip into its Item Branch and goes through Checks and review again.

Note (Milestone 5): the engine stopped naming the feature workflow, so the trailer is now `Surt-Item` and the merge message says `Merge item <key>` ([A. Git Integration](../architecture/A-git-integration.md#integration)). The decision is unchanged.
