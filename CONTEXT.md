# Surt

Surt is a local-first CLI that orchestrates AI agents through declarative workflows. The engine knows nothing about specs, tasks, builders or reviewers; those belong to the workflows a project keeps under `.surt/`, such as the ones the feature Template writes.

## Language

### Workflows

**Workflow Definition**:
A declarative, versioned description of steps and their dependencies: the project's file `.surt/workflows/<name>.toml`. It is read when a root Workflow Run that needs it is created and frozen with that run; the binary holds none.
_Avoid_: Workflow (alone), pipeline, Template (which only writes definitions)

**Template**:
A set of Workflow Definitions and Agents that `surt init --template <name>` writes once into `.surt/`, never overwriting a file that exists. Once written, the files are the project's to edit, rename or delete; a Surt upgrade never changes them. The binary knows only Templates, never a workflow; the one it has is `feature`.
_Avoid_: Built-in workflow, default workflow, preset

**Workflow Run**:
One live instance of a Workflow Definition, carrying its own state. It freezes its Workflow Definitions and Agents when it is created and never rereads their files.
_Avoid_: Workflow (alone), job, process

**Child Workflow Run**:
A Workflow Run started by a step of another Workflow Run, such as one per item of a Per-Item Step. It has its own state, so it can block, be cancelled or be resumed on its own.
_Avoid_: Sub-run, nested workflow

**Step**:
A node declared in a Workflow Definition.
_Avoid_: Stage, phase, node

**Step Run**:
One occurrence of a Step inside a Workflow Run, with its own state in the flow.
_Avoid_: Step (when the occurrence is meant), job

**Execution**:
One concrete launch of work for a Step Run, either an agent session or an Operation. A Step Run has zero or more Executions, at most one running at a time; an Approval Gate has none.
_Avoid_: Session, job, invocation

**Iteration**:
A new logical round inside a Workflow Run, triggered by a decision such as a rejected review. It creates new Step Runs; finished Step Runs are never reopened.
_Avoid_: Retry, loop, round

**Attempt**:
A new Execution of the same Step Run after a technical failure such as a crash or invalid output.
_Avoid_: Retry (ambiguous with Iteration), rerun

**Outcome**:
The single result a Step Run ends with when it succeeds, taken from a closed set its step declares, such as `approved` or `changes_requested`. Workflow conditions only ever compare Outcomes. A technical failure is not an Outcome; it leads to a new Attempt.
_Avoid_: Result, status, decision (alone)

**Loop**:
A step that repeats a group of steps, its body, as new Iterations until one of them ends with an expected Outcome.
_Avoid_: Cycle, retry, back-edge

**Item List**:
An Artifact holding items with unique keys, dependency keys and optional named files. The list and its referenced files share one immutable Revision. It is the only shape a Per-Item Step expands; the engine does not know what the items mean.
_Avoid_: Manifest (the feature workflow's Item List), batch

**Per-Item Step**:
A step that starts one Child Workflow Run for each item of an Item List, in the order their dependencies allow, and finishes when all of them are terminal.
_Avoid_: For-each, fan-out, map

**Operation**:
Work Surt performs itself, without an AI, as an Execution, such as Git integration commands.
_Avoid_: Native operation, built-in step, script, hook

**Run Branch**:
The branch, owned by Surt, that belongs to one Workflow Run. A root Workflow Run's Run Branch collects its children's reviewed work; a Child Workflow Run's Run Branch is where that child works, starting from its root's Run Branch tip. Workflows may give them role-specific names.
_Avoid_: Work branch, output branch

**Item Branch**:
The Run Branch of a Child Workflow Run started by a Per-Item Step, `surt/item/<subject>/<key>` whatever the workflow calls its items. In the feature Template it is the branch a Builder commits to in its own worktree, starting from the Integration Branch's tip when its Task begins.
_Avoid_: Task Branch, topic branch

**Subject**:
The uniquely named thing a root Workflow Run works on. A Subject can accumulate several Workflow Runs over time (e.g. a retry after cancelling), but at most one is active at once.
_Avoid_: Target, context, project

**Approval Gate**:
A step that pauses a Workflow Run until a human decides about a specific Artifact Revision.
_Avoid_: Approval step, checkpoint

**Approval**:
A recorded human decision (approve or reject) on one Artifact Revision: who, when and which Revision. A new Revision never inherits the Approval of an earlier one.
_Avoid_: Sign-off, acceptance

### Running

**Driver**:
The single process that advances one root Workflow Run, together with all its Child Workflow Runs. A root Workflow Run has at most one Driver at a time, and none while it waits on a human.
_Avoid_: Runner, worker, daemon

**Agent Slot**:
One of the project's `max_concurrent_agents` places that an agent Execution must hold to run. Slots belong to the project, not to a Driver: every Driver of the project takes one before launching an agent and gives it back when the agent ends, so two Subjects driven at once share the limit.
_Avoid_: Worker, seat

**Interruption**:
Stopping a Driver, for example with Ctrl-C or a stop request from another terminal, without ending its Workflow Run. Running Executions are stopped, and the run can be resumed later with new Attempts.
_Avoid_: Cancel (which ends the run), abort, pause

**Cancellation**:
An explicit decision that ends a Workflow Run in a terminal state. Work starts again only as a new Workflow Run of the same Subject.
_Avoid_: Interrupt, stop, abort

**Recovery**:
What every Driver does when it takes over a Workflow Run: it stops work left running without a Driver and settles every pending Intention against the real state before anything new starts. It is not a separate command or mode.
_Avoid_: Repair, resume, doctor

**Intention**:
A record Surt commits before an effect outside its own state, such as moving a branch, and confirms or discards afterwards. Recovery compares Intentions with reality; it never guesses what an effect should have been.
_Avoid_: Lease, journal, pending operation

**Salvage Ref**:
A reference that keeps whatever an Attempt left behind in a worktree (uncommitted changes, unaccepted commits) before the worktree is reset for the next Attempt. It can be inspected, but its content never feeds another Attempt.
_Avoid_: Stash, backup

**Transient Failure**:
A technical failure caused by something outside the agent that passes by itself, such as a rate limit or a network outage. Surt pauses every agent launch of the Driver until the reported reset time or a backoff ends, then tries again without spending an Attempt.
_Avoid_: Retryable failure (which spends an Attempt), outage

**Blocked**:
A Step Run that cannot go on without a human, because it reached its Attempt or Iteration limit or hit a failure that retrying cannot fix. It is not terminal: running the driving command again resumes it. A Workflow Run never fails; it is blocked until resumed or cancelled.
_Avoid_: Failed, stuck, error

**Cleanup**:
What Surt removes after a Workflow Run ends: the views and scratch space it can rebuild or never accepted, such as the Working Copy and the item worktrees. It never removes history, logs, a human's unrecorded edit, or the Integration Branch, and a cancelled run keeps its unintegrated work as refs. A cleanup that could not finish is pending and is finished by the next command that takes over the run. A `succeeded` run whose effective policy is `on_success = "keep"` is kept instead: it has no cleanup until `surt delete`.
_Avoid_: Garbage collection, purge, archive

**Purge**:
Deleting a Subject for good: its runs' history, logs, worktrees and refs, except the Integration Branch of a run that succeeded. It is the only way history leaves Surt.
_Avoid_: Cleanup, soft delete, archive

### Agents

**Agent**:
A role an AI performs in a workflow: what it does, its instructions and the contract of what it must deliver. It is the project's directory `.surt/agents/<id>/` (`agent.toml`, `instructions.md`, optional `schemas/`), complete on its own: it is read when a root Workflow Run that uses it is created and frozen as an agent snapshot, with no defaults merged in.
_Avoid_: Persona, bot

**Agent Profile**:
The concrete way of running an Agent: runtime, model, effort, permissions and limits. Each Agent has exactly one, used by every Execution of it; repeating work never switches to another model.
_Avoid_: Model config, agent config

**Agent Runtime**:
The adapter that knows how to launch and follow one kind of agent, such as Claude Code or the Fake Runtime used in tests.
_Avoid_: Provider (reserved for external integrations), driver, backend

**Context Assembler**:
The component that prepares an Execution's context from its fixed Inputs, current item and delivery contract, alongside the Agent's stable instructions.
_Avoid_: Prompt builder, agent memory

### Artifacts

**Artifact**:
A named piece of content a workflow produces or consumes, such as a spec, a task manifest or review feedback.
_Avoid_: Document, file, output

**Artifact Revision**:
An immutable version of an Artifact. A recorded human edit, an agent regeneration or new feedback always creates a new Revision; nothing is mutated. Approval always targets one Revision.
_Avoid_: Version, edit, update

**Working Copy**:
The editable local view of the latest Revision of an Artifact that passes through an Approval Gate. Surt notices an edit and records it as a new human-origin Revision, unless the Artifact's approved Revision is pinned. Agents never read a Working Copy; they read Revisions.
_Avoid_: Checkout, draft, local file

**Pinned Revision**:
An approved Revision that some Execution has already read as an Input. For the rest of its Workflow Run, a human edit to that Artifact is refused rather than recorded; changing it means cancelling and starting a new run.
_Avoid_: Locked, frozen (reserved for definitions and agent snapshots)

**Input / Output**:
An Execution reads specific Artifact Revisions as Inputs, fixed when it starts, and produces new Revisions as Outputs. Every Revision comes from exactly one origin: one Execution or one human edit.
_Avoid_: Dependency (reserved for ordering between Steps)

**Initial Artifact**:
An Artifact whose first Revision is supplied by the command that creates a root Workflow Run, as a human-origin Revision, rather than produced by a step: one per Run Input given, such as the feature Template's request and Base.
_Avoid_: Parameter, argument, seed

**Run Input**:
An input a root Workflow Definition declares under `[inputs.<name>]`, with a format (`text`, `markdown`, `json` or `git_ref`), supplied with `--input <name>=<value>` on `surt run` (or `surt continue --new-run`) and stored as the Initial Artifact of the same name. A `git_ref` input is resolved to `{ref, sha}` when the run is created, from the value or its `default`. A workflow with no inputs runs without any.
_Avoid_: Parameter, argument, flag

### The feature Template (project files, not part of the engine)

What `surt init --template feature` writes: the workflows `feature` and `task-cycle` and the five Agents below. Once written they are files of the project like any other; these terms name what the Template defines, and a project that edits or replaces the files may use other words.

**Feature**:
The Subject of a run of the `feature` workflow, named `YYYYMMDD-<slug>` from its request.
_Avoid_: Project, epic, change

**Task**:
A unit of work listed in the Task Agent's manifest, with a key, dependencies and its own Markdown instructions for the Builder. The manifest and these documents are approved together. Only the feature Template knows Tasks; the engine sees them as items to expand over, each on its own Item Branch.
_Avoid_: Ticket, issue, story

**Task Cycle**:
The Workflow Definition `task-cycle` that builds and reviews one Task; each Task runs as its own Child Workflow Run of it.
_Avoid_: Task loop, task pipeline

**Spec Agent**:
The Agent that writes the spec from the user's request.

**Task Agent**:
The Agent that breaks an approved spec into the Task manifest.

**Builder**:
The Agent that implements one Task in its own worktree and commits.
_Avoid_: Coder, implementer

**Reviewer**:
The Agent that judges a Builder's commit and returns approved or changes requested, with findings.
_Avoid_: QA, checker

**Shipper**:
The Agent that writes the delivery text for a Feature's final Integration Branch. It only reads; it never commits or merges.
_Avoid_: Releaser, deployer

**Base**:
The commit a run starts from: the `feature` workflow's `git_ref` Run Input `base`, resolved when the run is created (`HEAD` by default). Its Integration Branch is created exactly there. The Git Operations read the Initial Artifact `base` whatever workflow runs them.
_Avoid_: Parent branch, upstream

**Integration Branch**:
The Run Branch of a Feature's root run: it collects every approved Task of one Feature run as a merge of its reviewed commit. It is what the Shipper delivers.
_Avoid_: Feature branch, release branch

**Conflict**:
The reason an approved Task could not enter its Integration Branch: its changes clash with Tasks merged since it started, or the combination fails the Checks. It sends the Task back to its Builder.
_Avoid_: Merge failure, rejection

**Check**:
A command declared by the project that Surt runs against a Builder's commit to verify it, such as tests or lint. Its results reach the Reviewer.
_Avoid_: CI, validation, hook
