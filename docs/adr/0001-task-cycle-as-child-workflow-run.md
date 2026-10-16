# Task Cycle runs as a Child Workflow Run

Each Task's build/review cycle runs as its own Child Workflow Run of the `task-cycle` Workflow Definition, started by the feature workflow's Per-Item Step, rather than as extra Step Runs flattened into the feature's Workflow Run (e.g. `B/build#2`). A Task needs its own state — blocked after too many rejections, cancelled, resumed — independently of its siblings, and a child run gives it that with a concept the engine already has; flattening would force a new "group of Step Runs" concept just to answer "what is the state of Task B?".

## Consequences

- The engine must support a parent/child relation between Workflow Runs, and a Per-Item Step Run finishes only when all its Child Workflow Runs are terminal.
- Iterations of the cycle create new Step Runs inside the child run; Step Runs are never reopened.
