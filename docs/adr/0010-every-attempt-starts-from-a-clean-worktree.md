# Every Attempt starts from a clean worktree; leftovers are salvaged, never continued

Before every Execution that works in a run's worktree, Surt checks out the run's branch at the last tip it accepted (`run_branch.tip_sha`), with no uncommitted changes, no untracked files that are not ignored, and no merge, rebase or similar in progress. Anything else it finds is first recorded in a commit under `refs/surt/salvage/<execution-id>`, then discarded. This happens for every Attempt, after a crash, an orphan, an Interruption or a plain `failed` Execution, and also when a human committed on an Item Branch. Ignored files, such as build caches, survive.

## Considered Options

- **Continue from the leftovers:** the next Attempt, a new Claude session with no memory, would inherit half-done edits, commits nobody validated and possibly a merge with conflict markers from a session whose stream was lost. Its result would depend on where the previous one died, and the evidence checks of a delivery (a new commit since the Attempt started) would compare against a moving starting point.
- **Block and let a human clean up:** safe, but every Ctrl-C and every invalid delivery would need manual Git work before the run could go on, which defeats running unattended.
- **Salvage, then reset (chosen):** every Attempt starts from the same state, the normal path and recovery run the same code, and nothing is lost: the leftovers stay inspectable with `git log -p refs/surt/salvage/<id>` until the run's cleanup.

## Consequences

- Work of an interrupted or failed agent Attempt never reaches the next Attempt, even when it was nearly done. Retrying costs a full Attempt.
- Item Branches belong to Surt while their run is active, like the Integration Branch. A human who wants to change a Task's code does it through Surt (a rejected review, a new Iteration), not by committing on its branch.
- Accepting an Execution that ran in a worktree must record the branch's tip as `tip_sha` in the same transaction. Otherwise the next preparation would reset to an older commit.
- Salvage Refs accumulate during a run, and the cleanup must remove them.
