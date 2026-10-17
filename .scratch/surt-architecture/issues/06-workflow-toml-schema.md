# Workflow TOML schema

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Assignee: Kaliel
Blocked by: 01

## Question

What is the smallest TOML schema that expresses the feature workflow and the task-cycle without turning into a language?

Decide: step types (agent, human_approval, foreach/subworkflow, native) and their fields; how **execution dependencies** (`needs`) × **data dependencies** (`inputs` referencing other steps' artifacts) are declared; how the foreach dynamically expands over Tasks that only exist after the Task Agent runs, respecting the inter-Task dependencies they declare themselves; how the subworkflow receives parameters and returns outputs; conditions/guards (e.g. the reviewer's decision) without arbitrary expressions; static validation (cycles, nonexistent inputs, step types); workflow versioning and the snapshot in the run; built-in workflows × overrides in `.surt/workflows/`; per-workflow cleanup policies.

Output: section E with complete `feature.toml` and `task-cycle.toml` that parse.

## Context

From [Domain glossary](01-domain-glossary.md): the `foreach` does not generate flattened Step Runs — each item starts a **Child Workflow Run** of a Workflow Definition (e.g. `task-cycle`), and the foreach's Step Run only finishes when all children are terminal (ADR 0001). An Iteration creates new Step Runs; a finished Step Run is never reopened — the TOML schema must express the loop without cycles in the DAG under that rule.

## Answer

Resolved in a grilling session. Document: `docs/architecture/E-workflow-schema.md` (with complete `feature.toml` and `task-cycle.toml` that parse); trade-off in `docs/adr/0003-inputs-resolved-by-artifact-name.md`; terms **Outcome**, **Loop**, **Item List**, **Per-Item Step** and **Operation** (renamed from Native Operation) in `CONTEXT.md`.

Decisions:
- **Not a language:** five step types (`agent`, `approval`, `operation`, `loop`, `per_item`), `kind` required; no expressions, templating, JSONPath or back-edges. `[steps.<id>]` syntax (id uniqueness enforced by TOML itself); unknown field = error.
- **Outcome:** every step ends with a value from a declared closed set (the agent's in `agent.toml`, `approved`/`rejected` for the gate, the operation's in Surt's registry; implicit `done`). `when`/`until` only compare Outcomes by equality. A false `when` → SKIPPED (a state, not an Outcome). A technical failure is not an Outcome.
- **Loop:** `kind = "loop"` with `body` and `until = { step, outcome }`; each Iteration creates new Step Runs for the whole body. **No `max_iterations` in the TOML** (the limit, if any, belongs to the loop ticket). A rejection at a gate = a new round of the gate's loop (`spec-loop`, `tasks-loop`).
- **Inputs by name, latest Revision** (ADR 0003), not "the Revision the gate approved". Protection against reading something unapproved moves to the artifacts ticket's revalidation: the driving command refuses (`✖ spec changed after approval (rev 2 not approved)`).
- **Contract in `agent.toml`:** `outcomes`, `[inputs]` (with `optional = true`) and `[outputs]`; the step does not repeat the list, it only renames when the names differ. Between Iterations the next agent reads the newest Revision by name (`feedback`, `spec-rejection-reason`).
- **Rejection reason** (`surt reject <feature> -m "…"`) becomes a human-origin Revision of the gate's `rejection_artifact`: `spec-rejection-reason`, `manifest-rejection-reason` (names per gate, otherwise one gate's reason leaks to the other gate's agent).
- **Explicit ordering with `needs`**, never inferred; body steps only depend on the same body, the Loop carries the external dependencies.
- **Per-Item Step** (`kind = "per_item"`, name chosen instead of `for_each`): `items` points to an **Item List** of fixed shape (`key` + `needs`, the rest is payload), validated when the agent delivers it (cycle/duplicate key → new Attempt). `pass = [...]` declares what the child sees from the parent (read-only); the parent does not read the children's artifacts (the Shipper uses the integration branch + `manifest`). There is no standalone subworkflow step.
- **Engine inputs:** `request` (text from `surt feature`, a human Revision) on the root; `item` (a reference to an item of the approved Item List, not a new Revision) on the child.
- **Freezing:** the root run freezes the closure (`feature` + `task-cycle`) at creation; identity = SHA-256 of the canonical form (`feature@3f9a2c`); `schema = 1` is the format version.
- **Cleanup:** only `[cleanup] on_success = "clean" | "keep"` (default `clean`); what "clean" removes belongs to the cleanup ticket.
- **Static validation** before any run is born (list of 8 checks in section E).
- **Agent ids:** `spec`, `tasks`, `builder`, `reviewer`, `shipper`.
- **Out of scope:** overriding/customizing built-in workflows and project-specific workflows (only the ones built into the binary exist; the schema does not prevent it).

## Comments

Later extension approved in [Agent contract](07-agent-contract.md): Item Lists accept per-item `files`, with formats declared in `agent.toml` and bytes included in the same approved Revision. Section E was updated; the mechanism stays generic and does not create a new Step kind. The detailed decision and rationale live in the contract ticket and in [ADR 0004](../../../docs/adr/0004-item-list-files-share-approved-revision.md).
