# Surt

Surt is a local-first CLI that orchestrates AI agents through declarative workflows. The engine knows nothing about specs, tasks, builders or reviewers; those belong to the built-in feature workflow that runs on top of it.

## Language

### Workflows

**Workflow Definition**:
A declarative, versioned description of steps and their dependencies. Built in or overridden per project.
_Avoid_: Workflow (alone), pipeline, template

**Workflow Run**:
One live instance of a Workflow Definition, carrying its own state. It freezes the effective Workflow Definition when it is created and never rereads it.
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
A new Execution of the same Step Run after a technical failure such as a crash, timeout or invalid output.
_Avoid_: Retry (ambiguous with Iteration), rerun

**Outcome**:
The single result a Step Run ends with when it succeeds, taken from a closed set its step declares, such as `approved` or `changes_requested`. Workflow conditions only ever compare Outcomes. A technical failure is not an Outcome; it leads to a new Attempt.
_Avoid_: Result, status, decision (alone)

**Loop**:
A step that repeats a group of steps, its body, as new Iterations until one of them ends with an expected Outcome.
_Avoid_: Cycle, retry, back-edge

**Item List**:
An Artifact holding a list of items, each with a unique key and the keys it depends on. It is the only shape a Per-Item Step expands; the engine does not know what the items mean.
_Avoid_: Manifest (the feature workflow's Item List), batch

**Per-Item Step**:
A step that starts one Child Workflow Run for each item of an Item List, in the order their dependencies allow, and finishes when all of them are terminal.
_Avoid_: For-each, fan-out, map

**Operation**:
Work Surt performs itself, without an AI, as an Execution, such as Git integration commands.
_Avoid_: Native operation, built-in step, script, hook

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

**Interruption**:
Stopping a Driver, for example with Ctrl-C, without ending its Workflow Run. Running Executions are stopped, and the run can be resumed later with new Attempts.
_Avoid_: Cancel (which ends the run), abort, pause

**Cancellation**:
An explicit decision that ends a Workflow Run in a terminal state. Work starts again only as a new Workflow Run of the same Subject.
_Avoid_: Interrupt, stop, abort

### Agents

**Agent**:
A role an AI performs in a workflow: what it does, its instructions and the contract of what it must deliver.
_Avoid_: Persona, bot

**Agent Profile**:
A concrete way of running an Agent: runtime, model, effort, permissions and limits. An Agent has several; one is chosen per execution.
_Avoid_: Model config, agent config

**Agent Runtime**:
The adapter that knows how to launch and follow one kind of agent, such as Claude Code or the Fake Runtime used in tests.
_Avoid_: Provider (reserved for external integrations), driver, backend

### Artifacts

**Artifact**:
A named piece of content a workflow produces or consumes, such as a spec, a task manifest or review feedback.
_Avoid_: Document, file, output

**Artifact Revision**:
An immutable version of an Artifact. A human edit, an agent regeneration or new feedback always creates a new Revision; nothing is mutated. Approval always targets one Revision.
_Avoid_: Version, edit, update

**Input / Output**:
An Execution reads specific Artifact Revisions as Inputs, fixed when it starts, and produces new Revisions as Outputs. Every Revision comes from exactly one origin: one Execution or one human edit.
_Avoid_: Dependency (reserved for ordering between Steps)

### Feature workflow (built-in, not part of the engine)

**Feature**:
The Subject of the built-in feature workflow, named `YYYYMMDD-description`.
_Avoid_: Project, epic, change

**Task**:
A unit of work listed in the manifest the Task Agent produces, with an id and dependencies on other Tasks. Only the built-in feature workflow knows Tasks; the engine sees them as items to expand over.
_Avoid_: Ticket, issue, story

**Task Cycle**:
The Workflow Definition that builds and reviews one Task; each Task runs as its own Child Workflow Run of it.
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
The Agent that delivers the final integrated branch, never merging it into the main branch.
_Avoid_: Releaser, deployer
