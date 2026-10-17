# Exhausted limits block a run instead of failing it

When a step reaches `max_attempts` failed Attempts, a Loop reaches `max_iterations` Iterations, or a failure cannot be fixed by retrying, the Step Run becomes `blocked`, a non-final state. It never becomes a final `failed`. The next driving command for the run moves it back to `running` with a fresh budget. A Workflow Run therefore ends only as `succeeded` or `cancelled`, and its other display states (running, awaiting approval, blocked, interrupted) are derived from its Step Runs instead of being stored.

## Considered Options

- **No limits:** a Builder and a Reviewer that disagree, or a delivery that is invalid every time, would spend tokens all night with nobody watching.
- **A final `failed` state for the step and the run:** a Feature with nine merged Tasks and one stuck Task would have to be cancelled and started over as a new Workflow Run. Finished Step Runs are never reopened, so "retry the failed one" would need a reopening rule that contradicts the domain model.
- **Block, and resume when a human runs the command again (chosen):** the limit stops unattended spending, and the human who looks at `status` decides whether to go on. Going on is just a new Attempt or Iteration of a Step Run that never finished.

## Consequences

- There is no `failed` Workflow Run or Step Run. "Gave up" is expressed only by `surt cancel`.
- A run can stay `blocked` indefinitely. Cleanup and status must treat it as active.
- Attempt and Iteration counters keep their full history. Only the budget is measured from the moment the step was last unblocked.
- A Loop whose body contains an Approval Gate has no Iteration limit, because a human already decides each of its Iterations.
