# Builder ↔ Reviewer loop state machine

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Blocked by: 06, 07, 09
Assignee: Kaliel

## Question

How is a Task's Builder ↔ Reviewer cycle modeled as a state machine (inside the subworkflow) without cycles in the DAG?

Decide: states and transitions of the task-cycle; how iteration and attempt increment; the iteration limit and what happens when it is exceeded (Task `BLOCKED`? workflow `BLOCKED`? human intervention?); how the Reviewer's findings become a feedback artifact consumed by the next Builder; profile escalation between iterations; what happens if the Builder produces no commit or the Reviewer returns invalid output; review of trivial changes after an integration conflict.

Output: part of section G.

## Context

From [Domain glossary](01-domain-glossary.md), already decided: the task-cycle is one Child Workflow Run per Task; **Iteration** = a new round by decision (rejected review), it creates new `build`/`review` Step Runs in the child run; **Attempt** = a new Execution of the same Step Run after a technical failure. Still missing here: states, transitions, the limits of each and what happens when they are exceeded.

From [Process model and concurrency control](05-process-and-concurrency-model.md): the state machine needs to name: an **interrupted** Execution (Ctrl-C/Interruption → new Attempt on resume), a **queued** Execution waiting for an agent slot, a pending **cancel request** on a run with a live Driver, and the terminal **Cancellation** state. Decide here whether an Interruption counts toward the Attempt limit.

From [Workflow TOML schema](06-workflow-toml-schema.md): the cycle is a `kind = "loop"` step (`body = ["build", "check", "review", "integrate"]`, `until = { step = "integrate", outcome = "merged" }`); `review = changes_requested` leaves `integrate` SKIPPED and creates a new Iteration; so does `integrate = conflict`. **The TOML has no `max_iterations`** by the user's decision: decide here whether a limit exists and where it is configured (or that there is none). The approval gates are also Loops (`spec-loop`, `tasks-loop`): rejection = new Iteration. Decide here what a BLOCKED/cancelled child of a `per_item` means for the items that depend on it (`needs`) and for the `per_item`'s Step Run.

From [Agent contract](07-agent-contract.md): frozen profiles `normal` and `reinforced`, with `normal` as default; a new technical Attempt keeps the profile, a new Iteration may select another. This ticket decides the escalation thresholds/policy. `changes_requested` with a valid delivery is a success with that Outcome; invalid JSON/files and reached configured limits are technical failures. No execution timeout. Builder/Reviewer receive the Task's `.md` at the manifest's approved Revision. See the [canonical contract](../../../docs/architecture/F-agent-contract.md).

From [Git integration model](09-git-integration-model.md) ([A. Git Integration](../../../docs/architecture/A-git-integration.md)): `check = failed` now skips `review` and `integrate` (`when` on `review`) and creates a new Iteration; the Builder reads `feedback?`, `check-report?` and `conflict?`, and the Context Assembler labels each Input with its originating Iteration/step and says why the Iteration started. `git.integrate = conflict` (`kind = merge | checks`) → the Builder merges the tip and goes through **full** check and review again (mandatory). Decide here: (1) whether the re-review after a conflict may focus only on the resolution delta (the Reviewer has the previously approved commit as an ancestor); (2) the reaction to a Builder with no new commit: today it is an invalid delivery → new Attempt under F's rule; decide whether it becomes its own Outcome; (3) whether the Iteration limit counts conflicts and failed checks the same as rejected reviews.

## Answer

Resolved by human exchange over three rounds. Complete section in [G. State Machines](../../../docs/architecture/G-state-machines.md); trade-off in [ADR 0008 — Exhausted limits block a run instead of failing it](../../../docs/adr/0008-limits-block-instead-of-failing.md). The glossary gained **Blocked**, and **Agent Profile** became one per Agent.

Final decisions:

- **Execution:** `queued → running → succeeded | failed | interrupted | cancelled`. Only `failed` counts as an Attempt; `interrupted` (Ctrl-C or a stopped orphan) and `cancelled` do not count. Operations are never `queued`.
- **Step Run:** `pending → running → done(Outcome)`, plus `skipped`, `blocked ⇄ running` and `cancelled`. `blocked` is not final, so leaving it reopens nothing: the same Step Run gets new Attempts or Iterations. An Approval Gate stays `running` without an Execution (shown as awaiting approval) and has no limit.
- **Workflow Run:** records only `active | succeeded | cancelled` + a cancel request. `RUNNING / AWAITING APPROVAL / BLOCKED / INTERRUPTED` are derived from the Step Runs on read. **There is no `failed` run** (ADR 0008).
- **Attempts:** `max_attempts = 3` per `agent`/`operation` Step Run. A failure that retrying cannot fix (auth, model, launch, Integration Branch moved externally) blocks on the 1st one. Network and rate limit are left to the recovery ticket.
- **Iterations:** `max_iterations = 5` per Loop, counting every cause (review, check, conflict). It does not apply to Loops with `approval` in the body (`spec-loop`, `tasks-loop`). Exhausting it takes the Loop to `blocked`.
- **Unblocking:** the driving command that takes the lock moves every `blocked` in the run back to `running` with a fresh budget; the counters keep the history. The limits are project config, frozen in the root run; their exact position in `config.toml` is left to the CLI.
- **Per-Item:** dependents of a blocked child stay `pending` and independent ones continue. There is no per-Task cancellation in M1, so `implement` ends `done` only with every Task integrated. Partial delivery is out of scope.
- **Single profile:** the `normal → reinforced` escalation was removed. Each Agent has a single `[profile]` (no `default_profile`, no names), with default `claude-sonnet-5-5` / `high` for all five. Attempts and Iterations always use the same profile. F, C, E and the glossary updated.
- **Builder with no change:** no new Outcome. The instruction is to make a justified empty commit, which goes through check, review and integrate. Forgetting to commit is still an invalid delivery and creates an Attempt.
- **Reviewer:** now reads `feedback?` (its own previous verdict: it checks the blocking findings and does not move the bar) and `conflict?` (focus on `git diff reviewed_commit..commit`). The verdict always covers the whole SHA; the focus on the delta is only an instruction.

Documentation validation: TOML blocks in A/E/F/G parse (`tomllib`). Context notes added to [SQLite schema](11-sqlite-schema.md), [Recovery and reconciliation strategy](12-recovery-and-reconciliation.md), [Cleanup and lifecycle safety](13-cleanup-delete-and-logs.md), [CLI surface and UX](14-cli-surface-and-ux.md), [Rust workspace structure and core traits](15-rust-workspace-and-traits.md) and [Test strategy](16-test-strategy.md). No new ticket.
