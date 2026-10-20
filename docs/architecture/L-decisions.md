# L. Architecture Decision Records

Index of the decisions that were hard to reverse, surprising without context and the result of a real trade-off. Each record lives in [`docs/adr/`](../adr/); this page only lists them. Every other decision is in the section it shapes (A–K) and in the ticket that resolved it on the [map](../../.scratch/surt-architecture/map.md). Decision ticket: [Roadmap + Milestone 1 proposal](../../.scratch/surt-architecture/issues/17-roadmap-and-milestone-1.md).

| ADR | Decision | Shapes |
|---|---|---|
| [0001](../adr/0001-task-cycle-as-child-workflow-run.md) | Each Task's build/review cycle is a Child Workflow Run of `task-cycle`, not Step Runs flattened into the feature run. | [C](C-domain-model.md), [E](E-workflow-schema.md), [G](G-state-machines.md) |
| [0002](../adr/0002-file-locks-over-sqlite-leases.md) | OS file locks, not SQLite leases, decide who owns a run, a slot, Git or a migration. | [A. Process Model](A-process-model.md) |
| [0003](../adr/0003-inputs-resolved-by-artifact-name.md) | Steps read artifacts by name at the latest Revision; revalidation, not wiring, keeps unapproved content out. | [E](E-workflow-schema.md), [H](H-artifact-storage.md) |
| [0004](../adr/0004-item-list-files-share-approved-revision.md) | An Item List Revision includes the bytes of every file its items reference. | [F](F-agent-contract.md), [H](H-artifact-storage.md) |
| [0005](../adr/0005-edits-to-pinned-revisions-are-refused.md) | An edit to an approved Revision already read as an Input is refused, not recorded. | [H](H-artifact-storage.md), [I. Cleanup](I-cleanup.md) |
| [0006](../adr/0006-merge-into-base-is-opt-in-fast-forward.md) | Merging into the Base is opt-in and fast-forward only. | [A. Git Integration](A-git-integration.md), [J](J-cli.md) |
| [0007](../adr/0007-integrate-reviewed-sha-as-merge-commit.md) | A Task is integrated as a merge commit of the exact reviewed SHA. | [A. Git Integration](A-git-integration.md), [I. Recovery](I-recovery.md) |
| [0008](../adr/0008-limits-block-instead-of-failing.md) | Exhausted limits block a step; a run ends only `succeeded` or `cancelled`. | [G](G-state-machines.md), [J](J-cli.md#exit-codes) |
| [0009](../adr/0009-history-is-purged-not-soft-deleted.md) | History is purged by cascade, never soft-deleted. | [D](D-database.md), [I. Cleanup](I-cleanup.md#surt-delete) |
| [0010](../adr/0010-every-attempt-starts-from-a-clean-worktree.md) | Every Attempt starts from a clean worktree; leftovers are salvaged, never continued. | [I. Recovery](I-recovery.md#preparing-a-worktree-for-every-attempt), [I. Cleanup](I-cleanup.md) |
| [0011](../adr/0011-two-crates-keep-the-engine-generic.md) | Two crates, so the compiler keeps the engine generic. | [B](B-workspace.md), [K](K-roadmap.md) |
| [0012](../adr/0012-agent-runtime-is-the-only-trait.md) | `AgentRuntime` is the only trait; store, Git and clock run for real in tests (a recorded exception to the DIP rule). | [B](B-workspace.md), [K](K-roadmap.md) |
| [0013](../adr/0013-definitions-live-in-the-project.md) | Workflows and Agents are files under `.surt/`; the binary only writes the feature Template and its CLI is generic. | [B](B-workspace.md), [E](E-workflow-schema.md), [F](F-agent-contract.md), [J](J-cli.md) |
| [0014](../adr/0014-a-conversation-continues-across-a-loop.md) | An Agent with `conversation = "loop"` continues the Claude conversation of its last successful Execution in the same Loop under a new session id; a lost transcript restarts it fresh, so correctness never depends on it. | [F](F-agent-contract.md), [I. Recovery](I-recovery.md), [E](E-workflow-schema.md), [J](J-cli.md) |

The Milestone 1 cuts of [K](K-roadmap.md) are not ADRs: each is a subset of v1 that needs no migration, so reversing one is just building it.
