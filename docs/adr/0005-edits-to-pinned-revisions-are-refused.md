# Edits to a pinned Revision are refused, not recorded

Once an approved Revision has been read as an Input by any Execution, it is pinned for the rest of its Workflow Run. A later edit to that artifact's Working Copy is refused: no Revision is created, `surt approve` refuses for that artifact, and a Driver that detects the edit launches nothing new, lets running Executions finish and exits. The human either restores the Working Copy or cancels and starts a new run. A reader would expect every human edit to become a Revision, as it does before consumption. We refuse instead because using the edit would silently break consistency: the manifest was derived from the old spec, and the Child Workflow Runs are already pinned to the old manifest. Re-expanding or re-planning in the middle of a run does not exist. Together with [ADR 0003](0003-inputs-resolved-by-artifact-name.md), this rule is what guarantees that no Builder works from content a human has not approved.

## Considered Options

- **Record the edit and allow reapproval after consumption:** later Builders would read the new spec, while the manifest and finished Tasks still come from the old one, with no signal to anyone.
- **Record the edit and ignore it:** the human's change would sit as the latest Revision that nothing reads, contradicting "steps read the latest Revision".

## Consequences

- Fixing a spec after planning has started means cancelling and starting a new run. Rejecting the manifest does not help, because the Task Agent rereads the pinned spec.
- `surt restore` must exist, to discard an unrecorded edit without creating a Revision.
- Pinning is generic engine state: it applies to any approved artifact of any workflow, not only to specs and manifests.
