# Spec: Milestone 4 — fit the project

Labels: ready-for-agent
Type: spec
Status: open (tickets not cut yet)
Map: [Surt — Architecture and implementation plan](../surt-architecture/map.md)
Plan: [K. Roadmap](../../docs/architecture/K-roadmap.md#after-milestone-1), [F. Built-in defaults and local overrides](../../docs/architecture/F-agent-contract.md#built-in-defaults-and-local-overrides), [A. Merging into the Base](../../docs/architecture/A-git-integration.md#merging-into-the-base-opt-in), [I. An interrupted fast-forward](../../docs/architecture/I-recovery.md#an-interrupted-fast-forward-into-the-base), [I. Cleanup — what each ending removes](../../docs/architecture/I-cleanup.md#what-each-ending-removes), [J. Configuration](../../docs/architecture/J-cli.md#configuration), [ADR 0006](../../docs/adr/0006-merge-into-base-is-opt-in-fast-forward.md)
Previous: [Milestone 3](../milestone-3/spec.md)

## Problem Statement

Milestones 1 to 3 deliver a Feature end to end, let the developer operate and debug it, and let it run unattended. Every project still gets the same Surt: the same five Agents with the same model, effort, tools and instructions, the same ending, and the same manual last step. Three things push back as soon as Surt is used on a real repository:

- **The agents do not fit the repository.** The Builder runs `claude-sonnet-5-5` at `high` effort with the built-in instructions. A team that wants Opus for the Builder and a cheaper effort for the Shipper, a turn or budget cap on a Builder that wanders, API-key auth on a CI machine, a Reviewer that knows the project's conventions, or a Builder allowed to run `npm test` cannot say so. `F` designs `.surt/agents/<id>/` overrides, and `surt init` even tells the user that `agents/` is versioned, but every driving command refuses with exit `3` the moment the directory exists.
- **Every Feature ends with the same manual merge.** The run succeeds with `surt/20261017-add-login` delivered, and the developer types `git merge --ff-only surt/20261017-add-login` on `main` every time, even when nothing moved on `main` since the Feature started and Surt already proved every commit was reviewed and checked. `[git] merge_into_base = "ff-only"` ([ADR 0006](../../docs/adr/0006-merge-into-base-is-opt-in-fast-forward.md)) refuses with exit `3`, `feature.toml` has no `merge` step, and `--merge`/`--no-merge` do not exist.
- **A successful run destroys what the developer wanted to look at.** Cleanup after `succeeded` removes the Task worktrees, the Task Branches, the Salvage Refs and the Working Copy. A developer tuning the overrides above wants to open the Builder's worktree after a good run, diff the Task Branches against the Reviewer's feedback, or rerun the project's tests in the exact tree that was merged. `on_success = "keep"` does not parse, and even where it would, it lives in a built-in Workflow Definition the user cannot change.

## Solution

Milestone 4 ("Fit the project") ships the three mechanisms K deferred to it. With them, Surt is **v1**: everything sections A–J describe.

```
$ cat .surt/agents/builder/agent.toml
[profile]
model = "claude-opus-5-5"
effort = "medium"

[profile.limits]
max_turns = 100

$ surt feature "add login" --merge
✔ created 20261017-add-login   base: main @ a11c9d0
  local agents: builder (agent.toml)
  merge into base: ff-only (--merge)
…
$ surt start add-login
✔ task A merged (3c1aa20)
✔ task B merged (9e04b7d)
✔ shipper    delivered — surt/20261017-add-login @ 4e1b2c0
✔ merged into main (fast-forward a11c9d0 → 4e1b2c0)
✔ run        succeeded
↻ cleanup    2 worktrees · 2 task branches · working copy
```

- **Agent overrides.** A project may put `agent.toml`, `instructions.md` and `schemas/*.schema.json` under `.surt/agents/<id>/` for any of the five built-in Agents. Surt merges `agent.toml` over the built-in one field by field (tables recursively, scalars and arrays replaced), replaces the instructions whole, overlays schemas by path, validates the effective Agent and the built-in workflows against it, and freezes the result into the new root run. Mistakes refuse at `surt feature` with the file, the field and the reason (exit `3`). Runs already created keep the Agents they froze.
- **Opt-in fast-forward into the Base.** `[git] merge_into_base = "ff-only"` in `.surt/config.toml`, or `--merge` on the command that creates a run, adds one step after the Shipper: Surt fast-forwards the Base branch to the delivered tip when nothing moved and the branch is not checked out in a dirty worktree, and otherwise says why it did not and leaves the merge to the developer. It never creates a merge commit on the Base branch. `--no-merge` turns it off for one run.
- **Keep a successful run.** `[cleanup] on_success = "keep"` in `.surt/config.toml` makes a `succeeded` run keep its worktrees, branches, Salvage Refs, Working Copy and Execution directories until `surt delete`. A cancelled run is unaffected.

Like Milestones 1 to 3, nothing here needs a migration: `ref_move` already carries `worktree_path`, `cleaned_at` already allows a kept run, the `config` snapshot already holds any per-run override, and `agent` snapshots already hold the effective Agent.

## User Stories

### Agent overrides

1. As a developer, I want to change the Builder's model in `.surt/agents/builder/agent.toml`, so that the Agent that writes code uses the model my team trusts for this repository.
2. As a developer, I want to change one Agent's effort without touching the others, so that the Shipper can run cheaper than the Builder.
3. As a developer, I want an override to name only the fields it changes, so that a two-line file is enough and every other field keeps the built-in value.
4. As a developer, I want a nested table such as `[profile.limits]` to merge key by key, so that setting `max_turns` does not discard the other profile fields.
5. As a developer, I want an array in an override (for example `allowed_tools`) to replace the built-in array whole, so that I know exactly which tools are approved without reading the binary's defaults.
6. As a developer, I want to add `Bash(npm test *)` to the Builder's `allowed_tools`, so that the Builder can run my project's tests under `dontAsk`.
7. As a developer, I want to set `max_turns` and `max_budget_usd` per Agent, so that a wandering Builder fails technically instead of running for hours.
8. As a developer on a CI machine, I want to switch an Agent to `auth = "api_key"` with `api_key_env`, so that Surt runs without an interactive subscription login.
9. As a developer, I want a local `instructions.md` to replace the Agent's built-in instructions entirely, so that the Reviewer can follow my project's conventions without fighting the defaults.
10. As a developer, I want to override a JSON Schema the Agent declares by placing a file with the same path under `schemas/`, so that I can tighten what the Reviewer must deliver.
11. As a developer, I want a schema I override to resolve its local `$ref`s against the same agent's schema directory, so that a split schema works like the built-in ones.
12. As a developer, I want a schema reference to a URL refused, so that freezing a run never depends on the network.
13. As a developer, I want the built-in schemas I did not override to stay in effect, so that overriding `agent.toml` alone does not break validation.
14. As a developer who misspells a field (`modle = …`), I want `surt feature` to refuse with the file, the field and the reason (exit `3`), so that a typo never silently falls back to the default.
15. As a developer who writes an unsupported effort or a non-positive limit, I want the same refusal, so that a bad profile is caught before any Claude session starts.
16. As a developer who creates `.surt/agents/buildr/`, I want a refusal naming the unknown Agent and the five known ids, so that a misspelt directory is not ignored.
17. As a developer who leaves an unexpected file (`instruction.md`, `notes.txt`) in an Agent's directory, I want a refusal naming it, so that a misnamed instructions file is not silently unused.
18. As a developer whose override sets `id` to something other than its directory name, I want a refusal, so that the directory and the contract never disagree.
19. As a developer who removes the Reviewer's `changes_requested` Outcome or renames an output a workflow consumes, I want a refusal naming the workflow step that no longer validates, so that an override can never produce a run that cannot finish.
20. As a developer, I want an override that removes an output the built-in delivery checks rely on (the Builder's `commit`, the Reviewer's `feedback`) refused, so that the guarantees "a Builder delivers a real commit" and "the Reviewer judged that commit" still hold.
21. As a developer using the release binary, I want an override that sets `runtime = "fake"` refused as test-only, so that a stray test profile never reaches production.
22. As a developer, I want overrides read and frozen when a root run is created (`surt feature "<description>"`, `surt feature <feature> --new-run`), so that the run's Agents are fixed for its whole life.
23. As a developer, I want edits to `.surt/agents/` during a run to leave that run untouched, including its later Iterations and Attempts, so that changing the Builder for the next Feature never changes the Feature already being built.
24. As a developer, I want `surt plan`, `surt start`, `surt approve` and the other commands to stop refusing when `.surt/agents/` exists, so that a project with overrides is a normal project.
25. As a developer, I want the creation line to list which Agents use local files and which files, so that I can see at a glance that my override was picked up.
26. As a developer, I want `surt inspect <execution>` to keep showing the model and effort the Execution ran with, so that I can confirm the override reached Claude Code.
27. As a developer, I want an empty `.surt/agents/` or an empty Agent directory to mean "no override", so that `git` placeholder directories cause no error.
28. As a team, I want `.surt/agents/` versioned with the repository, so that every developer and the CI run the same Agents.
29. As a developer upgrading Surt, I want my local files never overwritten and the built-in defaults to change only for new root runs, so that an upgrade never changes a run in progress.
30. As a developer on Windows, I want overrides with CRLF line endings to parse and freeze with the same identity rules as on macOS and Linux, so that a checkout with `core.autocrlf` behaves like any other.

### Fast-forward into the Base

31. As a developer, I want `[git] merge_into_base = "ff-only"` in `.surt/config.toml` to fast-forward the Base branch to the delivered tip after the Shipper, so that a Feature lands on `main` with no manual step when nothing moved.
32. As a developer, I want `surt feature "<description>" --merge` to turn the fast-forward on for this run only, so that I can opt in per Feature without changing the project.
33. As a developer, I want `--no-merge` to turn it off for this run only, so that a risky Feature stays on its Integration Branch even in a project that merges by default.
34. As a developer, I want `--merge` and `--no-merge` on `surt feature <feature> --new-run` too, so that a new run of a Subject chooses again.
35. As a developer, I want `--merge` with `--no-merge` refused as a usage error (exit `2`), so that the choice is never ambiguous.
36. As a developer who passes `--merge` when resuming the spec phase of an existing run, I want a refusal (exit `3`) saying the run already froze its choice and how to start a new run, so that I never think a flag took effect when it did not.
37. As a developer, I want the effective choice (project value or per-run flag) frozen into the run, so that editing `.surt/config.toml` later never changes what an existing run does.
38. As a developer, I want the creation line to say `merge into base: ff-only` and whether a flag chose it, so that I know before the Builders start whether `main` will move.
39. As a developer, I want `surt status` to show the frozen merge choice, so that I can check it at any time.
40. As a developer, I want the fast-forward skipped when the Base branch moved since the Feature started, with a line naming both SHAs and telling me to merge the Integration Branch myself, so that Surt never combines my Feature with commits it never checked.
41. As a developer, I want it skipped when the Base was a detached commit or a tag, so that `--base v1.2.0` never tries to move a tag.
42. As a developer with the Base branch checked out in my worktree and uncommitted changes there, I want it skipped with a line saying the worktree is not clean, so that Surt never touches files I am editing.
43. As a developer with a merge, rebase, cherry-pick or revert in progress in that worktree, I want it skipped the same way, so that Surt never steps into an operation I left half done.
44. As a developer with the Base branch checked out in a clean worktree, I want Surt to run the fast-forward inside that worktree, so that my files and my branch move together and my checkout is never left behind its ref.
45. As a developer with untracked files in that worktree that the fast-forward does not touch, I want the merge to proceed, so that a scratch file does not cost me the automation.
46. As a developer whose untracked file would be overwritten by the fast-forward, I want it skipped as not clean, so that Surt never destroys a file it does not know.
47. As a developer with the Base branch not checked out anywhere, I want the ref moved with a compare-and-swap, so that a branch moved by someone else at the same instant is never overwritten.
48. As a developer, I want a skipped fast-forward to still end the run `succeeded` (exit `0`), so that a moved `main` is a normal ending, not a failure.
49. As a developer with `merge_into_base = "never"`, I want no extra line at all, so that the default output is unchanged.
50. As a developer who presses Ctrl-C twice during the fast-forward in my worktree, I want the next command to block with the exact commands to finish or undo it (exit `4`), so that Surt never guesses with my worktree.
51. As a developer who finished the interrupted fast-forward by hand, I want the next `surt start` to see the ref at the tip and record the merge, so that I do not have to tell Surt what I did.
52. As a developer who undid it, I want the next `surt start` to try the fast-forward again, so that the automation still happens.
53. As a developer whose Driver was killed between recording the intended ref move and moving it, I want the next command to finish or skip it by reading Git, so that a crash never leaves `main` in a state Surt cannot explain.
54. As a developer, I want the fast-forward to run before cleanup and before the run is recorded `succeeded`, so that a run is only `succeeded` once its last step decided.
55. As a developer running `surt status` on a finished run, I want a line saying whether the Base was fast-forwarded (from → to) or why not, so that I know whether `main` already has the Feature.
56. As a developer running `surt status --json`, I want the merge result (`merged` or `skipped` with its reason) in the run document, so that a script can decide whether to open a pull request.
57. As a developer running `surt inspect <feature>`, I want the merge step on the timeline with its result, so that I can tell afterwards why `main` did or did not move.
58. As a developer with runs created before M4, I want them to finish exactly as before (no merge step), so that upgrading the binary never moves `main` for a run that did not ask for it.
59. As a developer, I want Surt never to push, never to create a merge commit on the Base branch and never to rewrite it, so that ADR 0006 holds.
60. As a developer on Windows, I want a fast-forward that fails because a file is in use to end as an interrupted or not-clean case with instructions, so that a locked file never leaves the worktree half updated without telling me.

### Keeping a successful run

61. As a developer tuning my overrides, I want `[cleanup] on_success = "keep"` in `.surt/config.toml`, so that a successful run leaves its worktrees, Task Branches and Salvage Refs for me to inspect.
62. As a developer, I want a kept run to also keep its Working Copy and Execution directories, so that I can see exactly what each Agent received and delivered.
63. As a developer, I want the policy frozen at run creation, so that switching it in the config affects new runs only.
64. As a developer, I want the end of a kept run to print one line saying it was kept and how to remove it, so that kept worktrees never pile up unnoticed.
65. As a developer, I want `surt status` and `status --all` to show `kept` for such a run, and `status --json` to report `cleanup.state = "kept"`, so that a kept run never looks like a pending or failed cleanup.
66. As a developer, I want no command to treat a kept run as cleanup pending, so that `surt start` on it does not suddenly remove what I kept.
67. As a developer, I want `surt delete <feature>` to remove a kept run like any other, so that there is one way out.
68. As a developer, I want a cancelled run to follow its own cleanup regardless of `keep`, so that the policy only describes successes.
69. As a developer starting `--new-run` on a kept Subject, I want run `N+1` to start without removing the kept run's worktrees and branches, which already have their own `.N` names, so that I can compare the two runs.
70. As a developer starting `--new-run` on a kept Subject, I want the previous run's Working Copy files that equal their baseline cleared and an unrecorded edit refused as today, so that two runs never share a Working Copy.
71. As a developer who writes `on_success = "keeep"`, I want every command to refuse with the file, the line and the reason (exit `3`), so that a typo never silently cleans what I wanted kept.

### Milestone close-out

72. As a Surt maintainer, I want the M4 suite to pass in CI on macOS, Linux and Windows, so that the three mechanisms hold on every MVP platform.
73. As a Surt maintainer, I want an M4 section of the manual Claude Code smoke on macOS (a Builder override with another model, a fast-forward into `main` of the toy repo, a kept run), so that M4 is checked against the real runtime.
74. As a Surt maintainer, I want K, A, E, F, I, J, D, B and `CONTEXT.md` updated to what was built, and K to say v1 is complete, so that the plan still wins on divergence.
75. As a Surt maintainer, I want every Milestone 1 refusal for these three mechanisms removed with its tests, so that no code path still says "not supported yet".

## Implementation Decisions

The plan documents ([F](../../docs/architecture/F-agent-contract.md#built-in-defaults-and-local-overrides), [A](../../docs/architecture/A-git-integration.md#merging-into-the-base-opt-in), [I. Recovery](../../docs/architecture/I-recovery.md#an-interrupted-fast-forward-into-the-base), [I. Cleanup](../../docs/architecture/I-cleanup.md), [E](../../docs/architecture/E-workflow-schema.md), [J](../../docs/architecture/J-cli.md)) already decide most of M4, and this spec consolidates them. Points the plan left open are marked **decided here**. The implementation writes them back into the plan document named.

### Scope boundary with M3

- M4 ships the three rows of K's "Mechanisms deferred" table marked M4. It does not depend on any M3 mechanism; it only adds keys to the project configuration that M3 also reads. K orders M3 before M4, and nothing here changes that order.

### Agent overrides

- **Where they are read.** The engine reads `.surt/agents/` when a root run is created (`Project::create_run`) and nowhere else. It merges each `<id>/` directory over the `Catalog`'s `AgentAssets` for that id, then validates and freezes exactly as for the built-ins. The CLI does not read the directory; the engine does not know which bytes came from the binary ([B](../../docs/architecture/B-workspace.md)). The "refuse when `.surt/agents/` exists" check on every driving command, its `AgentOverrides` refusal and their tests go away.
- **Merge rules** ([F](../../docs/architecture/F-agent-contract.md#built-in-defaults-and-local-overrides)): TOML tables merge recursively by key; an absent local field inherits; a local scalar or array replaces the built-in value whole. No deletion syntax and no CLI-argument passthrough. A local `instructions.md` replaces the whole built-in text. The merged document must satisfy the complete `agent.toml` schema with `deny_unknown_fields` at every level, then pass the existing profile checks (effort syntax, limits positive and finite, `api_key_env` only with `auth = "api_key"`).
- **Schemas.** **Decided here:** a schema path in the effective `agent.toml` resolves to the local file `.surt/agents/<id>/<path>` when it exists, otherwise to the built-in asset of the same path; a path found in neither is a refusal. Local schemas' `$ref`s are followed transitively within the same agent directory with the existing local-reference rules; a network or absolute reference is a refusal. The closure is frozen with the Agent.
- **Directory hygiene (decided here).** A directory under `.surt/agents/` that is not a built-in Agent id is a refusal listing the known ids. Inside an Agent directory only `agent.toml`, `instructions.md` and files under `schemas/` are allowed; anything else is a refusal naming the file. An empty directory (or an absent one) is no override. A local `agent.toml` that sets `id` must set it to its directory name. Files are read as UTF-8; CRLF is accepted, and the snapshot identity is computed over the canonical effective TOML, so line endings in `agent.toml` do not change it ([F. Freezing agents](../../docs/architecture/F-agent-contract.md#freezing-agents)); instruction and schema bytes are frozen as read.
- **Runtime `fake` (decided here).** An effective profile with `runtime = "fake"` is accepted only when the engine is built with `test-support`; otherwise it is a refusal ("test-only runtime").
- **Workflow validation.** After merging, the built-in workflow closure is validated against the effective Agents as today (Outcomes named by `until`/`when`, inputs and outputs by name, Item List files). **Decided here:** the `Catalog`'s delivery checks (`CommitEvidence`, `ReviewedCommit`, `NoBlockingFinding`) are part of that validation: an effective Agent that no longer declares the output a delivery check names, or the Outcome `NoBlockingFinding` names, is refused. An override cannot add a new Agent id.
- **Refusal shape.** A new `InvalidAgentOverride` refusal carries the file path, the field path (`profile.limits.max_turns`) or the workflow step when relevant, and the reason; J's message shape, exit `3`. It is raised by `create_run`, so `surt feature "<description>"` and `--new-run` refuse before any row is written.
- **Visibility (decided here).** `create_run` reports which Agents used local files and which ones (`agent.toml`, `instructions.md`, `schemas/…`); the creation message prints `local agents: builder (agent.toml), reviewer (instructions.md)` under the `created` line, and nothing when there are none. No column records the origin: the snapshot stores the effective Agent only, and `inspect <execution>` keeps showing the model and effort from `runtime_info`.
- **`surt init`** is unchanged: it still does not materialize built-in Agents, and still lists `agents/` as versioned.

### Fast-forward into the Base

- **Configuration.** `[git] merge_into_base` accepts `"never"` (default) and `"ff-only"`. The `MergeIntoBaseUnsupported` refusal and its tests go away, and the `surt init` template drops "not supported yet".
- **Per-run override.** `--merge` and `--no-merge` exist on `surt feature`, mutually exclusive in clap (exit `2`). They apply when the command creates a root run: the description form and `--new-run`. **Decided here:** when the command resumes an existing run instead, either flag is a refusal (exit `3`) naming the run's frozen value and pointing to `--new-run`. The engine receives the override as part of `NewRun` and freezes the effective value into the `config` snapshot ([D](../../docs/architecture/D-database.md#snapshots)); the engine does not know the flags.
- **The step.** `feature.toml` gains `[steps.merge]` (`kind = "operation"`, `run = "git.merge_into_base"`, `needs = ["ship"]`) as E already writes it. The definition SHA changes; runs frozen before M4 have no `merge` step and finish as before.
- **The Operation** `git.merge_into_base` reads `base`, `integration` and the frozen `merge_into_base`, and follows A's table, in this order:
  1. `never` → `skipped` (`disabled`).
  2. `base.ref` is `null` → `skipped` (`no_base_branch`).
  3. `refs/heads/<base.ref>` is not `base.sha` → `skipped` (`base_moved`).
  4. The branch is checked out in a worktree (found with `git worktree list --porcelain -z`): if that worktree is not clean, `skipped` (`worktree_not_clean`); else `git -C <worktree> merge --ff-only <tip>` → `merged`.
  5. Not checked out: `git update-ref refs/heads/<base.ref> <tip> <base.sha>` under `git.lock` → `merged`; a compare-and-swap failure → `skipped` (`base_moved`).
- **"Clean" (decided here):** no staged or unstaged changes to tracked files and no operation in progress (the `MERGE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD` and rebase markers of [research 04](../surt-architecture/issues/04-git-worktree-merge-partial-states.md)). Untracked files do not make it unclean. If `git merge --ff-only` itself refuses because it would overwrite an untracked file, the Outcome is `skipped` (`worktree_not_clean`); Git checks this before touching anything.
- **Intention.** Before moving anything the Operation writes its `ref_move` row (`ref_name`, `old_sha = base.sha`, `new_sha = tip`, `worktree_path` when step 4 applies), as I and D already describe, and deletes it in the transaction that records the Outcome. The ref move rule of [I](../../docs/architecture/I-recovery.md#ref-moves) settles it: ref at `new_sha` → `merged`; at `old_sha` → run again, except that with `worktree_path` set and that worktree not clean the Step Run becomes `blocked` (`not_retryable`, `merge_into_base_interrupted`), exit `4`, with I's finish/undo lines; anything else → `skipped` (`base_moved`). A Windows file-in-use failure of `git merge --ff-only` is a technical failure whose next Attempt lands in the same rules.
- **Recorded result (decided here).** The skip reason and the moved SHAs are written with the step's ending event: `step.done` for `git.merge_into_base` carries `outcome` plus `reason` (`disabled`, `no_base_branch`, `base_moved`, `worktree_not_clean`) on `skipped`, or `from`/`to` on `merged`; on `base_moved` it also carries `found`. D's per-kind field table gains these fields. They are for display only; the engine never decides from them.
- **Output (decided here).** `merged` prints `✔ merged into main (fast-forward a11c9d0 → 4e1b2c0)`. `base_moved`, `no_base_branch` and `worktree_not_clean` print a `⚠ not merged: …` line naming the reason and `merge surt/<subject> yourself`. `disabled` prints nothing. Every skip keeps exit `0`.
- **`status`.** The detail already shows the frozen `merge into base` mode; a run whose `merge` step ended adds the result line (`merged into main a11c9d0 → 4e1b2c0`, or `not merged: main moved (a11c9d0 → b22e310)`). `status --json` adds a `merge` object (`outcome`, and `reason`, `from`, `to`, `found` as recorded), additively under `"schema": 1`. `inspect` shows the step like any Operation.
- **Hooks.** Surt runs `git merge --ff-only` in the user's worktree as Git would, so the repository's `post-merge` hook runs; its exit status does not change the Outcome. `update-ref` records the reflog message `surt: fast-forward <base.ref> to surt/<subject>`.
- **Ordering.** `merge` runs after `ship`, before the run is recorded `succeeded` and before cleanup, so I's "the Driver, after the Shipper's delivery and the opt-in fast-forward" holds.

### Keeping a successful run

- **Configuration (decided here).** The switch is a project configuration key, `[cleanup] on_success = "clean" | "keep"` in `.surt/config.toml`, absent by default. Workflow Definitions keep their `[cleanup] on_success` field, which now parses `"keep"` too ([E](../../docs/architecture/E-workflow-schema.md)). The **effective policy** of a root run is the configuration's value when set, else its root Workflow Definition's (`clean` for the built-in `feature`). Child definitions' values are not consulted: cleanup is per root run ([I](../../docs/architecture/I-cleanup.md#when-cleanup-runs)). The effective policy is frozen into the `config` snapshot at creation. No per-run flag.
- **Ending.** A `succeeded` run with `keep` runs no cleanup step, leaves `cleaned_at` NULL and is **kept**, not cleanup pending. Status derives `kept` from the frozen policy, so the existing `CleanupState::Kept` and its text (`kept (on_success = "keep") — remove with: surt delete <subject>`) become reachable. The Driver prints that line instead of the `↻ cleanup` line and writes no cleanup event.
- **Cancelled** runs ignore the policy ([I](../../docs/architecture/I-cleanup.md#what-each-ending-removes)). **`surt delete`** removes a kept run exactly as I's `surt delete` column says.
- **`--new-run` on a kept Subject (decided here).** The kept run's worktrees and branches already carry their own `.N` names and stay. Before creating run `N+1`, the command clears the previous run's Working Copy files that equal their baseline (the same rule as cleanup step 1) and refuses, as today, when one holds an unrecorded edit. The Working Copy belongs to the Subject's current run.
- **Recovery** is unchanged: a kept run is terminal, and reconciliation of another run of the same Subject never touches the kept run's paths or refs.

### Documentation updates

- **K:** the three M4 rows of "Mechanisms deferred" say they are implemented, the M4 row of "After Milestone 1" changes status like M2's and M3's, and K states that v1 is complete once M4 ships.
- **F:** schema resolution order, directory hygiene, `fake` outside `test-support`, delivery checks in validation, the creation line.
- **A:** the "clean" definition, the reason values, hooks and the reflog message, the output lines.
- **E:** `on_success` parses `keep`; the effective policy comes from the configuration first.
- **I. Cleanup:** the effective policy, the kept ending line, `--new-run` on a kept Subject. **I. Recovery:** unchanged apart from linking the reason values.
- **J:** `[cleanup] on_success` in the configuration table; `--merge`/`--no-merge` on `--new-run` and their refusal on resume; the creation lines; the `merge` result in `status` and `status --json`.
- **D:** the `step.done` fields for `git.merge_into_base`; `on_success` in the `config` snapshot.
- **B:** `workflow` merges overrides read by `create_run`; the refusals that no longer exist.
- **`CONTEXT.md`:** a new term **Agent Override** (files under `.surt/agents/<id>/` merged over a built-in Agent when a root run is created; _Avoid_: custom agent, plugin). The **Cleanup** entry gains that a `succeeded` run may be kept.
- **Smoke checklist:** an M4 section in the manual Claude Code smoke.

## Testing Decisions

- **A good test checks external behaviour through the highest seam that can observe it**: rows, events, snapshots and Git refs through the `Project` facade, exit codes and terminal output through the binary. It never asserts on private merge or Operation state. Real Git is used wherever Git decides, as in the existing worktree and ref-move tests. No new seam is introduced.
- **Main seam: `Project::create_run` + `Project::drive` with the Fake Runtime and real Git**, as in the existing engine drive tests (worktrees, ref moves, cleanup, recovery).
  - **Overrides:** a Builder override changing `model`, `effort` and `[profile.limits]` is frozen into the run's `agent` snapshot as the merged document, and every Builder Execution of that run (later Iterations and Attempts included) gets that profile, even after the override file is edited mid-run; an array replaces, a table merges; a local `instructions.md` replaces the text the Context Assembler hands to the runtime; a local schema overlays the built-in by path and a delivery that violates it is rejected; each refusal case (unknown field, bad effort, unknown agent directory, stray file, mismatched `id`, missing or network schema, removed Outcome, removed delivery-check output) refuses `create_run` with `InvalidAgentOverride` and writes no row; an empty directory changes nothing; CRLF and LF overrides freeze to the same snapshot identity for `agent.toml`.
  - **Fast-forward:** one test per row of A's table with a real repository: `never`, detached Base, moved Base, Base checked out in a dirty worktree, in-progress merge marker, clean checked-out worktree (files and ref move together), untracked file left alone, untracked file that would be overwritten, branch not checked out (CAS); the run ends `succeeded` in every case and `step.done` carries the recorded reason or SHAs. A per-run override frozen in `config` wins over the project file.
  - **Keep:** a `succeeded` run with `keep` leaves worktrees, Task Branches, Salvage Refs, Working Copy and Execution directories, `cleaned_at` NULL, and status `Kept`; driving it again removes nothing; `delete` removes it; a cancelled run with `keep` cleans as cancelled; `--new-run` on a kept Subject clears baseline Working Copy files and refuses on an unrecorded edit.
- **Recovery fault points** (existing seam): the `Intention` checkpoint for `git.merge_into_base`'s `ref_move`, and a crash after the ref moved but before the Outcome was recorded, both settle by reading Git (`merged`, run again, or `skipped`); the interrupted checkout case (ref at `old_sha`, `worktree_path` dirty) blocks with `merge_into_base_interrupted`, and the finish and undo paths each reach the expected Outcome on the next drive.
- **Config unit tests** (existing pattern): `merge_into_base = "ff-only"` parses; `on_success` accepts `clean`/`keep` and refuses anything else with path and line; the template parses.
- **Workflow parsing tests** (existing): a definition with `on_success = "keep"` parses and its canonical form keeps it; the built-in `feature.toml` with its `merge` step validates.
- **Binary tests** (`surt` crate CLI tests with TOML Fake scripts): `--merge` and `--no-merge` on creation and on `--new-run`; both together exit `2`; either on resume exits `3`; the creation lines (`local agents: …`, `merge into base: …`); the `✔ merged into …` and `⚠ not merged: …` lines and exit `0`; `merge_into_base_interrupted` exits `4`; `status` and `status --json` showing the merge result and a kept run; an invalid override refusing `surt feature` with exit `3`. The existing tests that expect `ff-only`, `on_success = "keep"` and `.surt/agents/` to refuse are replaced by these.
- **CI** runs fmt, clippy `-D warnings` and `cargo test --features test-support` on macOS, Linux and Windows, as before. Real Claude Code stays manual.
- **Manual macOS smoke** (new M4 section of the checklist): a Builder override to another model, confirmed with `surt inspect <execution>`; `surt feature … --merge` on a toy repository whose `main` is checked out and clean, ending with `main` at the delivered tip; a second Feature with `on_success = "keep"` whose worktrees are still there after `succeeded` and gone after `surt delete`.

## Out of Scope

- Overriding or customizing the built-in workflows, and project-specific workflows (map: out of scope). `on_success` is chosen in the project configuration, not by editing `feature.toml`.
- New Agent ids, named profiles, profile escalation, or a second real runtime (F, map).
- A deletion syntax for overrides, arbitrary CLI-argument passthrough, hooks, MCP tools, skills or custom agents in profiles (F).
- Materializing the built-in Agents with `surt init`, or a command that prints an Agent's effective definition.
- Recording which snapshot files came from local overrides, or warning when `.surt/agents/` changed since a run froze it.
- Merging into the Base by anything other than a fast-forward: no merge commit, no rebase, no squash (ADR 0006, ADR 0007); no push, no pull request.
- `--base` handling on the resume form of `surt feature`.
- A per-run `--keep` flag, partial keep policies, or removing a Task's worktree as soon as it is integrated (map: out of scope).
- Global (`~/.surt/config.toml`) keys for any of these three mechanisms.

## Further Notes

- **v1.** K says v1 is the end of M4. Once M4 ships, every mechanism sections A–J describe exists, and what the map ruled out of scope stays out.
- **ADR 0006 becomes code.** The map's standing constraint "no automatic merge into the main branch except the opt-in `ff-only`" is enforced by the Operation's compare-and-swap, not by an Agent's instructions; the Shipper stays read-only.
- **Writing into the user's worktree.** Recovery's principle "never writes to the user's worktree" has exactly one opt-in exception, the clean fast-forward of step 4, and it is the reason for the `worktree_path` Intention and the `merge_into_base_interrupted` block. Nothing else in M4 writes there.
- **Overrides and trust.** `.surt/agents/` is versioned, so pulling a branch can change which tools a new run's Builder may run. Tool and permission rules are not a sandbox (F); a team reviews `.surt/agents/` like any other code.
- **Shipping** follows Milestones 1 to 3: M4 ships once the suite passes in CI on the three OSes and the M4 section of the manual smoke passes on macOS. Tickets for M4 are cut from this spec into `.scratch/milestone-4/issues/`.
