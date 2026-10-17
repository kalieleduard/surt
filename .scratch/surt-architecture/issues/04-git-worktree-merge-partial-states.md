# Git worktree/merge mechanics and partial states

Map: [Surt — Architecture and implementation plan](../map.md)
Type: research
Status: resolved

## Question

What partial external states can Git leave behind when Surt is interrupted in the middle of an operation, and how can they be detected and reconciled idempotently?

Survey: `git worktree add/remove/prune/lock/repair` (with worktrees outside the repo, e.g. `~/.surt/worktrees/...`), what happens if the worktree directory disappears or the metadata in `.git/worktrees/` is orphaned; a branch that already exists/is checked out in another worktree; a leftover `index.lock`; interrupted merge/rebase/cherry-pick (`MERGE_HEAD`, `rebase-merge/`) and how to abort/detect them; worktree-free merges (`git merge-tree --write-tree` + `commit-tree` + `update-ref` with old-value) as an atomic alternative; `update-ref` with old-value verification for idempotency; getting the exact diff and SHA to bind a review to a commit; behavior on Windows (long paths, `core.longpaths`, symlinks, file locking that prevents removal); minimum required Git version; `git2`/`gix` × invoking the `git` binary.

Findings: `docs/research/git-worktree-merge.md` on the `research/git-worktree-merge` branch.

## Answer

Full findings: `docs/research/git-worktree-merge.md` on the `research/git-worktree-merge` branch (commit `9b2d8d1`; tested with git 2.56.0 on macOS; Linux/Windows from documentation only). Includes a 21-row "partial state → detection → safe reconciliation" table.

Facts that constrain the design (not decisions):
- `worktree add/remove/lock` and every `--abort` **fail if re-run after success** — idempotency has to come from Surt reading the state (`worktree list --porcelain -z`, `rev-parse -q --verify`) and comparing it with SQLite.
- `worktree add -b` is not atomic: Ctrl-C leaves the branch without a worktree; a hard kill leaves a `locked initializing` worktree with no index and an `index.lock` (`remove -f -f` fixes it; `prune` does not). A moved worktree becomes `prunable`; `git gc` prunes missing, unlocked worktrees after 3 months; deleted `.git/worktrees/<id>` metadata cannot be repaired.
- Git never removes stale locks; `index.lock` and merge/rebase markers are per worktree, ref locks are repo-wide. `core.lockfilePid` (2.56, off by default) records the owning PID. `status --no-optional-locks` keeps Surt's polling from taking the lock.
- `status --porcelain=v2` **does not signal** an in-progress merge/rebase — detect it through markers (`MERGE_HEAD`, `rebase-merge/`, `CHERRY_PICK_HEAD`, `sequencer/` …) via `rev-parse --git-path`.
- Worktree-free integration is viable and re-runnable: `merge-tree --write-tree` (0 clean, 1 conflict) → `commit-tree` with fixed dates (deterministic SHA) → transactional `update-ref --stdin`. Requires Git ≥2.38 (≥2.40 for `--merge-base`, ≥2.43 for `-X`). On conflict it still returns a tree with markers — never commit it.
- `update-ref` CAS: re-running after success fails; recovery rule: ref = new → done; = old → redo; anything else → stop. `update-ref` moves a branch checked out in another worktree without complaining (desynchronizing that worktree); `symref-update HEAD` without `no-deref` corrupts branches.
- Review binding: the commit SHA covers content + ancestry; `diff A...B` = `diff $(merge-base) B`. On Windows, what gets committed depends on `core.autocrlf`.
- libgit2 and gix do not replace the CLI (no `worktree repair`; libgit2's merge is not ort; `git2::Transaction` is not atomic; gix is in early development). Git for Windows ships with long paths and symlinks disabled.
