# Steps read artifacts by name, at the latest Revision

A step's inputs are resolved by artifact name to the latest Revision when its Execution starts. They are not wired to the Revision that an Approval Gate approved. A reader would expect the opposite, because approval targets one specific Revision. We chose names anyway: a human edit after approval is meant to be used rather than silently ignored, and Loops become simple. The Builder's second Iteration reads `feedback` and the Spec Agent's second Iteration reads `spec-rejection-reason`, without a "previous Iteration" reference syntax. As a result, the guarantee that nothing unapproved flows downstream does not come from the workflow wiring. It comes from the revalidation policy (section H): a driving command refuses while an artifact that passed a gate has a newer, unapproved Revision.

## Considered Options

- **Inputs wired to step outputs (`inputs.spec = "approve-spec.spec"`), with the gate passing on the approved Revision:** safe by construction and gives implicit ordering, but a human edit after approval would never be read, and loops would need a separate "read the previous Iteration" mechanism.

## Consequences

- Artifact names are scoped to one Workflow Run and must be unique per meaning. Two gates in one run each need their own `rejection_artifact`, or one gate's reason would reach the other's agent.
- Ordering cannot be inferred from data, so `needs` is always explicit.
- Section H's revalidation check is load-bearing for the brief's rule "no Builder before human approval".
