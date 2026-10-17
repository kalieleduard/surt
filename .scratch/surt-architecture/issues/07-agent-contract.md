# Agent contract

Map: [Surt — Architecture and implementation plan](../map.md)
Type: grilling
Status: resolved
Assignee: Kaliel
Blocked by: 01, 02

## Question

What is the complete contract of an agent — configuration, instructions, inputs, outputs and validation?

Decide: the `agent.toml` schema (id, runtime, model, effort, timeout, alternative profiles, permissions, tools, provider-specific parameters); how and when a profile is chosen (e.g. Sonnet High by default, Opus Medium after N rejections?); merging built-in defaults with local overrides and snapshotting the effective config; the role of `instructions.md` vs. the prompt assembled by the Context Assembler; how the agent declares and delivers outputs (files at agreed paths? a final JSON validated by schema?) — especially the Task manifest (ids, dependencies) and the Reviewer's structured decision (`APPROVED`/`CHANGES_REQUESTED` + findings); validating model/effort/flags against the installed Claude Code; the default permission posture for unattended execution in a worktree; the `AgentRuntime` trait and what the Fake Runtime needs to simulate.

## Context

Arose from [Claude Code CLI headless capabilities](02-claude-code-cli-headless.md) — also decide here:
- **Auth vs. isolation:** `--bare` (full isolation, requires an API key) vs. combined flags that keep the subscription login (`--setting-sources`, `--settings`, `--strict-mcp-config`, `CLAUDE_CODE_DISABLE_CLAUDE_MDS`) vs. a per-run `CLAUDE_CONFIG_DIR`. What is the default, and what goes into `agent.toml`?
- **Model/effort validation**, since the CLI does not validate before launch: our own versioned allowlist? A check after `system/init`?
- **Wall-clock timeout** enforced by Surt and how it relates to `--max-turns`/`--max-budget-usd`.
- **Minimum/pinned Claude Code version** and disabling auto-update during runs.
- **Success rule** for the execution (exit code + `is_error` + `subtype` + `structured_output`).

From [Workflow TOML schema](06-workflow-toml-schema.md): `agent.toml` owns the contract. It declares `outcomes = [...]` (a closed set; an agent with a single result has the implicit Outcome `done`), `[inputs]` (name → `format`, `optional = true` when it may be missing, e.g. `feedback` in round 1) and `[outputs]` (name → `format`). The workflow does not repeat the list: Surt delivers the latest Revision of each artifact with the same name, with optional renaming in the step when the names do not match. Static validation of the workflow loads the agents.

## Comments

### Round 1 — choices confirmed

The human answered `Q1 OK`, `Q2 OK`, `Q3 - OK`:

- **Authentication:** the existing subscription login by default, explicitly disabling inherited configuration where the CLI allows it; partial isolation acknowledged. API key with `--bare` as an explicit option.
- **Delivery:** artifacts in files plus a mandatory final JSON declaring the Outcome and the delivered files; Surt validates both. A file without valid JSON is a technical failure; a valid `changes_requested` is a success with that Outcome.
- **Freezing:** when the root run is created, freeze the effective contracts, instructions and profiles of every agent used, after local overrides. Each Execution records the profile chosen from the frozen set; local changes apply to new runs.

Incremental section: [F. Agent Contract](../../../docs/architecture/F-agent-contract.md). The complete contract is not resolved yet; the ticket stays assigned for the next rounds.

### Round 2 — overrides, profiles and permissions confirmed; timeout excluded

The human answered `Q4 - OK`, `Q5 - OK`, `Q6 - OK`, `Q7 - Don't add a timeout now`:

- **Overrides:** built-in defaults plus a per-field local override; lists are replaced wholesale; unknown fields are errors. A local `instructions.md` replaces the entire built-in text.
- **Profiles:** explicit names and default in the agent; a new Attempt after a technical failure keeps the profile; a new Iteration may select another frozen profile by loop policy. Thresholds belong to [Builder ↔ Reviewer loop state machine](10-builder-reviewer-loop.md). The runtime does not silently switch models to work around an error.
- **Permissions:** `dontAsk` with explicit tools and authorizations per profile. An unauthorized call is denied; if it prevents delivery, it is a technical failure with a diagnostic. `bypassPermissions` is an explicit alternative. No promise of an operating-system sandbox.
- **No execution timeout for now:** no field, default or total-duration timer in the current contract. This revises the earlier forecast in the process model. `stop_grace_period` still applies after Interruption/Cancellation; it is not an execution timeout. Turn and cost limits are not decided yet.

Decisions incorporated into [F. Agent Contract](../../../docs/architecture/F-agent-contract.md), with a consistency fix in [A. Process Model and Concurrency](../../../docs/architecture/A-process-model.md).

### Round 3 — context, envelope, model and limits confirmed

The human answered `OK for all` to questions Q8–Q11:

- **Instructions and context:** `instructions.md` holds the stable role; the Context Assembler assembles each Execution's Inputs with fixed Revisions, current item, working directory and delivery contract. No templates or expressions in the instructions.
- **Single envelope:** `{"outcome":"approved","outputs":{"feedback":"feedback.json"}}`; names and Outcomes come from the contract; paths are relative to the Execution's delivery directory; no escapes or symbolic links. Surt validates formats/schemas and only accepts a complete delivery; any inconsistency is a technical failure.
- **Model/effort:** full IDs, no moving aliases; local validation of known effort values; check the model reported at initialization. Incompatibility/unavailability is a technical failure, without picking another model. No per-account catalog of our own; the requested effort is recorded, with no promise to verify the effort actually applied.
- **Limits:** optional turns and cost per profile, with no default limits; sent to the CLI only if configured. Reaching a limit is a technical failure; cost is approximate and does not guarantee a maximum charge. Still no execution timeout.

Incremental section updated in [F. Agent Contract](../../../docs/architecture/F-agent-contract.md).

### Round 4 — deliverables, profiles, runtime and compatibility confirmed

The human confirmed Q12–Q15, adding: `each task will need its own .md to pass to the builder later`.

- **Deliverables:** Spec delivers Markdown; Tasks delivers an Item List with `key`, `needs`, `title`, `description`, `acceptance_criteria`; Builder delivers a SHA in JSON; Reviewer delivers the reviewed commit, a summary and findings (`blocking`, `path`, `message`, optional `line`); Shipper delivers branch, final SHA and summary in JSON. The Outcome stays in the envelope. `approved` with a blocking finding is inconsistent. Besides the manifest, each Task requires its own `.md`; packaging/freezing those files is still to be decided in the next round.
- **Built-in profiles:** `normal` (default) Sonnet/high; `reinforced` Opus/medium for all five agents. Full IDs confirmed from an official source: `claude-sonnet-5-5` and `claude-opus-5-5`; account availability is still checked at runtime.
- **Runtime:** launches, follows and stops an Execution; Surt validates the delivery and advances the flow. The Fake Runtime produces simulated files/events through the same validation path, without changing states in SQLite directly.
- **Compatibility:** minimum baseline 2.1.301, features checked and the effective version recorded per Execution; later versions accepted when compatible, with no exact pinning per run. Auto-update disabled in launched processes. Inspection confirmed the flags, without a full authenticated run; the real fixture is still pending in its own ticket.

Section [F. Agent Contract](../../../docs/architecture/F-agent-contract.md) consolidated with the TOML schema, JSON schemas and invariants. The ticket stays assigned while the delivery of per-Task `.md` files is settled.

### Round 5 — Markdown per Task confirmed

The human answered `THAT'S IT` to Q16: the Task Agent writes each `.md`, the item references it through `files.task`, and the manifest plus files form a single approved Revision. The Builder receives its own Task's file, fixed to the approved Revision. A missing/invalid file prevents delivery; an edit creates a new Revision and requires re-approval. The core handles generic attachments, not Tasks.

## Answer

Resolved by human exchange over the five rounds above. Complete contract in [F. Agent Contract](../../../docs/architecture/F-agent-contract.md), schemas in [schemas/](../../../docs/architecture/schemas/), the generic Item List extension in [E. Workflow Definition Schema](../../../docs/architecture/E-workflow-schema.md) and the trade-off in [ADR 0004 — Item List files share the approved Revision](../../../docs/adr/0004-item-list-files-share-approved-revision.md). Glossary and sections A/C aligned.

Final decisions:

- **Configuration and freezing:** built-in defaults + per-field local override, lists replaced, unknown fields rejected; a local `instructions.md` replaces the full text. The root run freezes agents, contracts, profiles, instructions and schemas; each Execution records the chosen profile.
- **Profiles:** `normal` Sonnet/high and `reinforced` Opus/medium for all five agents, with full IDs. A technical Attempt keeps the profile; another Iteration may choose a different one by loop policy. Thresholds remain in the loop ticket.
- **Auth and permissions:** subscription login by default, with explicit partial isolation; API key + `--bare` as an alternative. `dontAsk`, explicit tools/rules per profile; bypass only when explicit. No promise of an OS sandbox.
- **Delivery:** files + a mandatory JSON envelope `outcome`/`outputs`; independent validation of the envelope, files, formats/schemas and consistency. A valid `changes_requested` is a success; a technical failure is not an Outcome.
- **Tasks:** one `.md` written by the Task Agent for each item, through `files.task`. JSON and files share the Revision/Approval; each child receives only its own files. No agent-specific tables/logic branches in the core.
- **Limits:** no execution timeout for now. Optional turns/cost, with no default; `stop_grace_period` remains only the explicit stop sequence.
- **Context and runtime:** stable instructions kept separate from the assembled context with fixed Inputs. The runtime launches/follows/stops an Execution; the Fake produces files/events through the same validation path, without manipulating SQLite state.
- **Compatibility and acceptance:** CLI >=2.1.301 with compatible controls, version per Execution, auto-update disabled in children and a new session UUID per launch. Acceptance requires a compatible init, exit 0, an error-free final result and a valid delivery; tolerate new events/fields and drain the stream to the end.

Documentation validation: TOML/JSON examples parsed, JSON schemas parsed and local links checked. No runtime was implemented and no authenticated session was run; [Capture a real Claude Code stream as a fixture](19-real-claude-code-stream-fixture.md) remains open to validate the real combination. Retention/redaction was specified as a new ticket [Log, retention and redaction policy](13-cleanup-delete-and-logs.md) (later absorbed into 13), which now blocks cleanup. Persistence/revalidation, Git evidence, loop policy and the representation of the traits remain in the existing tickets.
