# Merging into the Base is opt-in and fast-forward only

The original brief forbids automatic merges into the main branch. We relax that to an opt-in: `[git] merge_into_base = "ff-only"` in `.surt/config.toml` (default `"never"`, frozen at root run creation) lets the Operation `git.merge_into_base`, after `ship`, fast-forward the Base branch to the delivered tip. It never creates a merge commit there. It skips when the Base branch has moved since the Base was fixed, when the Base is not a branch, or when the branch is checked out in a worktree that is not clean. When the branch is checked out in a clean worktree, Surt runs `git merge --ff-only` inside that worktree instead of moving the ref underneath it. The option belongs to the Operation, not to the Shipper's `agent.toml`, and the Shipper stays read-only.

## Considered Options

- **Never merge (the brief as written):** safest, but every Feature ends with the same manual step the user wants automated.
- **Let the Shipper agent merge:** it would need write access to Git, and "only fast-forward, only a clean worktree" would become prompt instructions instead of code with a compare-and-swap.
- **Create a merge commit when the Base branch moved:** the result combines the Feature with commits it was never checked or reviewed against, contradicting the invariant that every integrated tip passed the Checks.
- **Put the switch in the Shipper's `agent.toml`:** the engine would have to read an agent's contract to decide what an Operation does, coupling engine code to a built-in agent id.

## Consequences

- With the option on, a Feature can land on `main` with no human action after the task approval. Every commit it brings was still reviewed and passed the Checks on the exact tree being fast-forwarded.
- `skipped` is a successful Outcome. The user merges the Integration Branch themself, as when the option is off.
- The map's standing restriction becomes "no automatic merge into the main branch unless the project opts in with `merge_into_base = "ff-only"`".
