# F. Agent Contract

Decision ticket: [Agent contract](../../.scratch/surt-architecture/issues/07-agent-contract.md). Vocabulary is defined in [`CONTEXT.md`](../../CONTEXT.md). The workflow's use of agent contracts is defined in [E. Workflow Definition Schema](E-workflow-schema.md).

This section records the accepted agent contract, including the per-Task Markdown files delivered with the approved manifest. It defines the engineering plan, not an implemented runtime.

## Authentication and inherited configuration

The default Claude Code execution uses the developer's existing subscription login. It does not require a separately billed API key. Surt explicitly disables inherited configuration and context discovery where the CLI supports doing so, rather than relying on Claude Code's defaults.

This mode provides partial isolation. It must not be described as a sandbox or as complete isolation from user, project or managed configuration. The launch policy and compatibility checks are specified below.

An explicit API-key mode uses `--bare` for stronger isolation. It is an alternative to the default subscription mode. The authentication choice is explicit; merely finding an API key in the environment must not silently change a subscription execution into an API-billed execution.

The existing research found that `--bare` does not reuse subscription OAuth credentials, and that changing `CLAUDE_CONFIG_DIR` also changes credential lookup. A fresh config directory is therefore not a way to preserve the existing login automatically. Research context: [Claude Code CLI headless capabilities](../../.scratch/surt-architecture/issues/02-claude-code-cli-headless.md), with its full report on branch `research/claude-code-cli-headless`.

## Delivery: files and a structured result

An agent writes its output Artifacts to files and delivers a required final JSON object declaring the Outcome and the files delivered. The agent contract owns the permitted Outcomes and the declared inputs and outputs; the workflow does not duplicate them.

Surt validates both the final JSON and the referenced files before accepting the Execution. A process that exits successfully or says that it finished has not, by that fact alone, delivered valid output.

| Delivery | Interpretation |
|---|---|
| Valid final JSON and valid required files | Eligible for acceptance after the runtime success checks |
| Files exist, but final JSON is missing or invalid | Technical failure; the Step Run has no successful Outcome |
| Final JSON is valid, but a required file is missing or invalid | Technical failure; the Step Run has no successful Outcome |
| Valid delivery with Outcome `changes_requested` | Successful Execution with that Outcome; it is not a technical failure |

Every agent delivers the same envelope shape; its concrete JSON Schema is generated from the frozen agent contract. For example:

```json
{
  "outcome": "approved",
  "outputs": {
    "feedback": "feedback.json"
  }
}
```

The Outcome must belong to the contract's closed set, and output names must match the contract. Paths are relative to this Execution's delivery directory. Paths that escape the directory, symbolic links and missing or invalid required files are rejected. Surt validates each file against its declared format and, where applicable, schema, and only accepts a complete valid delivery.

Concrete file schemas and runtime success checks are specified below. The Claude Code `structured_output` shape and the failure streams below were observed in the real-stream fixtures (`docs/research/fixtures/claude-code/` on branch `research/claude-code-cli-headless`), captured with Claude Code 2.1.301 by [Capture a real Claude Code stream as a fixture](../../.scratch/surt-architecture/issues/19-real-claude-code-stream-fixture.md).

## Freezing agents

At creation, a root Workflow Run freezes every Agent used by the closure of its Workflow Definitions, including agents used by Child Workflow Runs. The snapshot contains each effective agent contract, its instructions and its effective Agent Profile after local overrides have been applied.

Every Execution of an Agent runs with that Agent's frozen Agent Profile and records which snapshot it used. Editing local model settings or instructions after run creation affects new root runs; it does not change an existing run, a later Iteration or a new Attempt of that run.

The snapshot also includes every local JSON Schema referenced by the contract. Its identity is a SHA-256 over the canonical effective TOML and the contents of its instructions and schema assets; absolute source paths are not part of that identity. Runtime credentials are not copied into it.

This extends the definition freezing rule in [E. Workflow Definition Schema](E-workflow-schema.md). It freezes Surt's definitions, not the external account, managed policies or installed Claude Code binary. Version compatibility is specified below.

## Built-in defaults and local overrides

Surt combines built-in defaults with explicit project-local overrides, field by field. Local lists replace the entire corresponding list; they are not appended. Unknown fields are errors. A local `instructions.md` replaces that agent's entire built-in instruction text rather than appending to it.

TOML tables merge recursively by key. An absent local field inherits the built-in value; a scalar replaces it, and an array replaces the complete array. There is no deletion syntax or arbitrary CLI-argument passthrough. Validate the resulting effective definition before freezing it.

An override may change an agent contract, but the resulting built-in workflows must still pass static validation. Changing an output name or Outcome does not implicitly rewrite a workflow that consumes it.

Project-local files are optional:

```text
.surt/agents/<agent-id>/
  agent.toml
  instructions.md
  schemas/<name>.schema.json
```

The binary contains complete defaults for `spec`, `tasks`, `builder`, `reviewer` and `shipper`. Local files replace or override those defaults; Surt does not overwrite them on upgrade. A schema path is relative to its agent's asset directory. Only local schema assets are supported: no network references. Freeze the transitive closure of local schema references too. Revisions are stored as content-addressed blobs in SQLite ([H. Artifact Storage, Freezing and Revalidation](H-artifact-storage.md)); the physical tables, including agent snapshots, are in [D. Database Schema](D-database.md#snapshots).

**When they are read.** `Project::create_run` reads `.surt/agents/` when a root run is created (`surt feature "<description>"`, `surt feature <feature> --new-run`) and nowhere else; driving commands never look at the directory, and a run keeps the Agents it froze whatever happens to the files afterwards. The merged `agent.toml` must satisfy the complete schema with `deny_unknown_fields` at every level and the profile checks of [Model and effort validation](#model-and-effort-validation) and [Effective `agent.toml`](#effective-agenttoml). Any failure refuses the creation with `InvalidAgentOverride` (exit `3`, no row written), which carries the project-relative file, the field path when one applies (`profile.limits.max_turns`) and the reason:

```
$ surt feature "add login"
✖ .surt/agents/builder/agent.toml: profile.effort: unknown variant `huge`, expected one of `low`, `medium`, `high`, `xhigh`, `max`
  fix: edit .surt/agents/builder/agent.toml
```

A local `agent.toml` with CRLF line endings freezes to the same identity as its LF version, because the snapshot stores the canonical effective TOML.

**Directory hygiene.** Every entry directly under `.surt/agents/` must be a directory named by a built-in Agent id; anything else (a misspelt directory, a stray `builder.toml`) refuses with the entry's path and the list of known ids. Inside an Agent directory only `agent.toml`, `instructions.md` and files under `schemas/` are allowed; any other file, at any depth, refuses naming it, with `fix: rename or remove <file>`. Empty directories carry no files, so an empty `.surt/agents/` or an empty `<id>/` (a `git` placeholder) is no override. A local `agent.toml` that sets `id` must set it to its directory name.

**Instructions and schemas.** A local `instructions.md` is frozen as read and is the text the Context Assembler hands to the runtime. A schema path in the effective `agent.toml` resolves in this order: the local file `.surt/agents/<id>/<path>` when it exists, otherwise the built-in asset of the same path; a path found in neither refuses with `InvalidAgentOverride` naming the file that declared it. Schemas the project did not override stay in effect, so overriding `agent.toml` alone keeps the built-in validation. Every local schema's `$ref`s are followed transitively with the same resolution order, relative to the referring schema's directory; a reference with a scheme (`https://`, `file:`), an absolute path or a path that leaves the agent's directory is refused naming the local schema, so freezing a run never touches the network. Every local schema must also be valid JSON that compiles as a Draft 7 JSON Schema against that closure; one that does not refuses with `InvalidAgentOverride` naming the local schema before anything is written, rather than at the first delivery. The closure is frozen with the Agent, and a delivery is validated against it from the snapshot alone.

**Test-only runtime.** An effective profile of an overridden Agent with `runtime = "fake"` is accepted only when the engine is built with the `test-support` feature; a release build refuses it at `profile.runtime` as a test-only runtime, so a stray test profile never reaches production.

**Workflow validation and delivery checks.** After merging, the built-in workflow closure of the new root run is validated against the effective Agents exactly as for the built-ins: Outcomes named by `until` and `when`, inputs and outputs by name, Item List files. The `Catalog`'s delivery checks are part of that validation: every Agent a workflow step runs must declare each output a delivery check names as a `json` output, and the Outcome `NoBlockingFinding` names. A built-in that fails is an `InvalidDefinition`; when only the effective Agents fail, the refusal is `InvalidAgentOverride` on the override that breaks it and names the workflow step that no longer validates, with the field of that step that needs the Agent (`when`, `until`, `input` or `delivery check`). The overrides are applied one Agent at a time, in Agent id order, over the built-ins; the first Agent whose override makes the closure fail is blamed, whether it overrides `agent.toml`, `instructions.md` or a schema, and the refusal names that Agent's `agent.toml` when it has one, otherwise its first local file:

```
$ surt feature "add login"
✖ .surt/agents/reviewer/agent.toml: a delivery check needs the outcome "approved"
  needed by: task-cycle step review (delivery check)
  fix: edit .surt/agents/reviewer/agent.toml
```

An override that leaves a `when` or `until` waiting for an Outcome the Agent no longer declares reads `outcomes no longer include "<outcome>"`. Because tables merge by key and there is no deletion syntax, an override can replace `outcomes` but cannot remove an input or output; it can still add a required input nothing supplies, or change the format of an output a delivery check reads, and both refuse the same way.

**The creation line.** `create_run` reports which Agents used local files and which ones. The creation line of `surt feature "<description>"` and of `--new-run` lists them, and prints nothing when there are none:

```
$ surt feature "add login"
✔ created 20261017-add-login   base: main @ a11c9d0
  local agents: builder (agent.toml), reviewer (instructions.md, schemas/feedback.schema.json)
```

## Agent Profile

An Agent has exactly one Agent Profile, the `[profile]` table of its `agent.toml`. It describes how to execute the Agent: runtime, model, effort, permissions and optional limits. There are no named profiles, no default-profile selection and no escalation:

- A new Attempt after a technical failure uses the same profile.
- A new Iteration uses the same profile too. A Loop that keeps failing is stopped by the Iteration limit and becomes blocked ([G. State Machines](G-state-machines.md#limits)); it never switches to a larger model.
- The Agent Runtime does not silently switch models to work around failures.

Different Agents may still use different models: each Agent's profile is configured, and overridden locally, on its own. Decision: [Builder ↔ Reviewer loop state machine](../../.scratch/surt-architecture/issues/10-builder-reviewer-loop.md).

## Permissions and tools

The default permission mode is `dontAsk`, with explicit tools and permission rules per Agent Profile. Rules are suited to the role: a Builder needs to edit code, run checks and commit, whereas a Reviewer needs to inspect the code and deliver its findings.

A tool call that lacks permission is denied rather than prompting the user. If denial prevents a valid delivery, the Execution fails technically with a diagnostic. `bypassPermissions` is an explicit alternative, not the default. Tool and permission rules are not an operating-system sandbox.

Tool availability and call approval are separate profile fields. Each effective Claude profile declares both `tools` and `allowed_tools`; `disallowed_tools` takes precedence. The launcher adds a non-overridable denial of MCP tools for this MVP. Profiles expose no hooks, custom agents, skills or raw CLI arguments.

| Agents | Built-in tools | Default approvals |
|---|---|---|
| Spec, Tasks | `Read`, `Glob`, `Grep`, `Write` | Read/search and Write under this Execution's delivery directory |
| Reviewer, Shipper | `Read`, `Glob`, `Grep`, `Write`, `Bash` | Same file rules, plus `git status`, `git diff`, `git show`, `git log`, `git rev-parse` inspection commands |
| Builder | `Read`, `Glob`, `Grep`, `Write`, `Edit`, `Bash` | Read/search, Write/Edit under its worktree and delivery directory, and Bash for implementation, checks and commits |

The runtime expands the file permission roots from its trusted Execution paths, not from model-provided output paths. It renders those roots in Claude Code's platform-specific permission-rule syntax. This expansion is launcher behavior, not a template language available in TOML. Shell permissions do not provide filesystem or network confinement. Native Windows uses Git Bash for the built-in Bash-based profiles; a missing shell is an actionable launch error. Git prerequisites and the final Shipper behavior are in [A. Git Integration](A-git-integration.md). The Shipper stays read-only there: it inspects the Integration Branch from the project root and never writes to Git. Builders get `core.longpaths=true` on Windows through `GIT_CONFIG_*` environment variables, never through the repository's config.

Claude's path rules for `Write` use the `Edit(...)` permission namespace; do not emit a `Write(path)` rule and assume it grants access. The fixture capture confirmed it: a `Write(./delivery/**)` allow rule is ignored without a stream-visible warning, and every write is denied under `dontAsk`. Absolute POSIX roots use Claude's `//` prefix, for example `Edit(//Users/dev/project/delivery/**)`, rather than a single-slash project-relative rule. Render Windows roots using the CLI's Windows path semantics. These are adapter rules, not portable filesystem paths copied verbatim into the CLI. Explicit `--tools` restores `Glob` and `Grep` on platforms where the default tool set omits them. Primary references: [read/edit permissions](https://code.claude.com/docs/en/permissions#read-and-edit) and [tool reference](https://code.claude.com/docs/en/tools-reference).

## No execution timeout in the current plan

There is no Surt-imposed wall-clock execution timeout, timeout field or default duration in the current agent contract. An Execution does not fail merely because it has been running for a long time. The developer can stop it through Interruption or Cancellation.

The existing `stop_grace_period` still applies after an explicit stop: it bounds the interval between a graceful stop request and forced process-tree termination. It does not bound the duration of an Execution. See [A. Process Model and Concurrency](A-process-model.md).

Turn and estimated-cost limits are optional per profile, with no limits by default. Surt passes them to the CLI only when explicitly configured. Reaching either limit is a technical failure, not an Outcome. A cost limit is approximate and is not a guaranteed billing cap.

## Instructions and assembled context

`instructions.md` defines the Agent's stable role and instructions. The Context Assembler builds the context of one Execution: its Inputs with fixed Revisions, its current item when applicable, its working directory and delivery contract. The instruction text has no templates or expressions.

The assembler uses the frozen instruction text and the Inputs selected for this Execution. A new Iteration can therefore supply a newer feedback Revision without changing the Agent's instructions.

Inputs resolve to the latest Revision by name, so a later Iteration can still receive a Revision produced several Iterations earlier, such as an old `conflict` after a newer `feedback`. The assembler therefore labels every Input with the Iteration and step that produced it, and, from the second Iteration of a Loop on, states why the current Iteration started: which step of the previous Iteration ended with which Outcome, or was SKIPPED. This is generic engine state; the assembler does not know what a conflict or a review is.

```
## Why this Iteration started
Iteration 3 ended: review = changes_requested
## Inputs
- feedback   (iteration 3, review)
- conflict   (iteration 2, integrate)
```

## Model and effort validation

Profiles name models by full IDs, not moving aliases. Surt validates known effort values locally and checks the model reported during runtime initialization against the requested model. An incompatible or unavailable model is a technical failure; Surt does not select a replacement model. It does not maintain its own catalog of models available to each account.

The requested effort is recorded, but Surt does not claim to verify the effective effort: the existing research did not find it exposed to headless consumers. Surt must not describe syntactic validation as proof of model-specific effort support. Supported syntax is `low`, `medium`, `high`, `xhigh` or `max`; an unsupported value is a configuration error rather than a warning passed through to Claude.

All five agents have the same built-in profile settings:

| Runtime | Model | Effort | Auth | Permission mode |
|---|---|---|---|---|
| `claude-code` | `claude-sonnet-5-5` | `high` | `subscription` | `dontAsk` |

Their tools follow the role table above and no profile sets turn or cost limits. These are pinned defaults of this plan, not aliases or a promise that every account can access them. A future Surt release may change defaults for new root runs; an existing run keeps its frozen IDs. IDs were checked against the official [model overview](https://platform.claude.com/docs/en/models/overview); the [Claude Code model configuration](https://code.claude.com/docs/en/model-config) documents model and effort behavior.

## Effective `agent.toml`

`schema = 1` is the contract format version. Agent IDs, input names and output names are non-empty lowercase identifiers containing letters, digits and hyphens, starting with a letter. Outcomes also permit underscores, as in `changes_requested`. Unknown fields at any nesting level are errors. A local override may omit fields; the effective definition must satisfy the complete schema.

| Field | Contract |
|---|---|
| `schema`, `id` | Required; known format, ID matching its agent directory |
| `outcomes` | Optional non-empty unique array; omitted means `["done"]` |
| `inputs.<name>` | `format` required; `optional` defaults to `false`; optional `json_schema` |
| `outputs.<name>` | `format` required; optional `json_schema`; every declared output is required |
| `outputs.<name>.item_files.<file-name>` | Only for `format = "item_list"`; required `format`, optional `json_schema`; every declared file is required on every item |
| `profile` | Required; `runtime`, full `model` and `effort`; optional `[limits]`; `[claude]` required for `claude-code` |
| `profile.limits` | Optional positive integer `max_turns` and positive finite numeric `max_budget_usd`; absent means no limits; no timeout field |
| `profile.claude` | `auth`, `permission_mode`, `tools`, `allowed_tools`, `disallowed_tools`; optional `api_key_env` only with `auth = "api_key"` |

Formats are `text`, `markdown`, `json` and `item_list`. Text and Markdown require non-empty UTF-8 content; Markdown is not semantically validated. JSON is parsed and validated against its declared JSON Schema when present. `item_list` also validates keys, references, graph acyclicity and declared item files as specified in section E. Item file formats are `text`, `markdown` or `json`; item files do not recursively contain Item Lists. `json_schema` is only valid for `json` or `item_list`. Schemas use draft-07; formats such as `date-time` do not substitute for explicit validation.

The supported runtime IDs are `claude-code` and test-only `fake`. The latter uses scripted behavior; it is not another model provider. Auth is `subscription` or `api_key`; permission mode is `dontAsk` or explicit `bypassPermissions`. Tool and rule arrays are explicit; an empty array is meaningful. No arbitrary settings or environment table is accepted. Claude-specific controls live in `[claude]`, keeping them out of the generic runtime request's contract semantics.

Complete effective Reviewer example (permission roots are added by the launcher as described above):

```toml
schema = 1
id = "reviewer"
outcomes = ["approved", "changes_requested"]

[inputs.item]
format = "json"

[inputs.spec]
format = "markdown"

[inputs.commit]
format = "json"
json_schema = "schemas/commit.schema.json"

[inputs.check-report]
format = "json"
json_schema = "schemas/check-report.schema.json"

[inputs.feedback]
format = "json"
json_schema = "schemas/feedback.schema.json"
optional = true

[inputs.conflict]
format = "json"
json_schema = "schemas/conflict.schema.json"
optional = true

[outputs.feedback]
format = "json"
json_schema = "schemas/feedback.schema.json"

[profile]
runtime = "claude-code"
model = "claude-sonnet-5-5"
effort = "high"

[profile.claude]
auth = "subscription"
permission_mode = "dontAsk"
tools = ["Read", "Glob", "Grep", "Write", "Bash"]
allowed_tools = ["Read", "Glob", "Grep", "Bash(git status *)", "Bash(git diff *)", "Bash(git show *)", "Bash(git log *)", "Bash(git rev-parse *)"]
disallowed_tools = ["mcp__*"]
```

A project override can be much smaller:

```toml
[profile]
model = "claude-opus-5-5"
effort = "medium"

[profile.limits]
max_turns = 100
max_budget_usd = 5.0
```

It still inherits the built-in profile's other fields; the final merged document is what gets validated and frozen. Built-in input/output contracts are:

| Agent | Inputs (`?` = optional) | Outputs | Outcomes |
|---|---|---|---|
| `spec` | `request: text`, `spec?: markdown`, `spec-rejection-reason?: text` | `spec: markdown` | `done` |
| `tasks` | `spec: markdown`, `manifest?: item_list`, `manifest-rejection-reason?: text` | `manifest: item_list`, including one required `task: markdown` item file per Task | `done` |
| `builder` | `item: json` with its read-only `task` Markdown file, `spec: markdown`, `feedback?: json`, `check-report?: json`, `conflict?: json` | `commit: json` | `done` |
| `reviewer` | `item: json` with its read-only `task` Markdown file, `spec: markdown`, `commit: json`, `check-report: json`, `feedback?: json`, `conflict?: json` | `feedback: json` | `approved`, `changes_requested` |
| `shipper` | `manifest: item_list`, `integration: json` | `delivery: json` | `done` |

`request`, `base` and `item` are supplied as defined in section E. `check-report`, `conflict` and `integration` are produced by Operations ([A. Git Integration](A-git-integration.md#operation-registry)).

## Built-in output schemas and validation

These schemas belong to the built-in feature workflow, not to engine knowledge of particular agent roles:

- [Task manifest schema](schemas/manifest.schema.json): `key`, `needs`, `title`, `description`, `acceptance_criteria` and `files.task` per Task. The manifest Revision includes the referenced Markdown bytes, not just their paths.
- [Builder commit schema](schemas/commit.schema.json): a full commit `sha`.
- [Reviewer feedback schema](schemas/feedback.schema.json): `reviewed_commit`, `summary` and `findings`. Each finding requires `blocking`, `path` and `message`; optional `line` is a positive, one-based integer.
- [Shipper delivery schema](schemas/delivery.schema.json): `title` and `summary`, text ready to become a pull request description. The delivered branch and SHA come from the `integration` Input; the Shipper does not declare them.

Artifacts produced by Operations, which agents read: [base](schemas/base.schema.json), [check-report](schemas/check-report.schema.json), [conflict](schemas/conflict.schema.json) and [integration](schemas/integration.schema.json).

An approved review with a blocking finding is inconsistent and is rejected. The final envelope is the single source of the review Outcome; feedback does not repeat it. Role-specific consistency checks are registered by the built-in workflow through generic delivery validation, never through engine branches on agent IDs. Merely matching a SHA regex is not proof that a commit exists, was reviewed or is integrated. The exact Git evidence for `commit` and the SHA binding of `feedback` and `check-report` are in [A. Git Integration](A-git-integration.md#evidence-for-a-builders-commit).

## Claude Code launcher and compatibility

The minimum supported version is **2.1.301**, the baseline whose current help was inspected. This is a deliberate support floor, not a claim that every flag was introduced in that version or that an end-to-end execution has been tested. Newer versions are accepted when the adapter's required controls and initialization checks remain compatible. Record the actual version on every Execution; do not pin the binary for the lifetime of a run.

Before launch, resolve the executable without a shell, inspect `--version` and required controls, validate the profile and schema assets, and check the selected auth mode. Recheck after an executable/version change. Help output is supplementary: documented controls such as `--append-system-prompt-file` and `--max-turns` are absent from the inspected help, so absence there is not evidence of incompatibility. Use the documented baseline and initialization checks; an actual unknown-flag error is an actionable launch failure. Neither successful preflight nor a model-shaped ID proves account access; a runtime auth/model failure remains possible.

Every Execution gets a new UUID session ID. The launcher uses these controls (arguments are an argv vector, not an interpolated shell command):

```text
-p
--session-id <fresh-uuid>
--no-session-persistence
--output-format stream-json
--verbose
--model <full-model-id>
--effort <requested-effort>
--permission-mode <profile-mode>
--permission-prompts none
--tools <explicit-built-in-tools>
--allowedTools <effective-allow-rules>
--disallowedTools <effective-deny-rules>
--json-schema <generated-delivery-envelope-schema>
--append-system-prompt-file <frozen-instructions-file>
--settings <generated-settings-file>
--strict-mcp-config
--mcp-config <file-containing-an-empty-mcpServers-object>
--disable-slash-commands
```

Pass the assembled Execution context on stdin, close stdin afterwards and keep Claude's built-in system prompt, appending the Agent's stable instructions. Never use `--continue`, `--resume`, `--fork-session`, a shared session ID or a fallback-model chain. Add `--max-turns` and `--max-budget-usd` only for configured limits.

For subscription auth add `--safe-mode --setting-sources user` and preserve the developer's existing credential lookup directory. Do not create a fresh `CLAUDE_CONFIG_DIR`. The generated settings disable user hooks and automatic content-based model switching (`disableAllHooks = true`, `switchModelsOnFlag = false`); managed policy still applies. The child environment disables auto-update (`DISABLE_AUTOUPDATER=1`, `DISABLE_UPDATES=1`), automatic memory and CLAUDE.md loading. Build the child environment from an allowlist (home, path, user, temp directory, locale and the platform's required variables, such as `SYSTEMROOT`, `USERPROFILE` and `APPDATA` on Windows) plus the variables listed here, instead of removing known variables from the inherited one: in the fixture capture, `CLAUDE_CODE_SIMPLE` and `ANTHROPIC_BASE_URL` inherited from a parent Claude Code session hid a valid subscription login. Inherited model/effort overrides and API-key, gateway and cloud-provider auth selectors are therefore never passed through. Check auth selection rather than treating an inherited key or helper as permission to change billing mode; refuse an unexpected API/provider auth source with an actionable diagnostic. No credentials are copied into the frozen definition or command-line arguments.

For explicit API-key auth use `--bare` instead of `--safe-mode`, take the key from the named `api_key_env` and pass it in the child environment. Keep generated settings and the explicit tool/MCP/skill controls. Do not claim that bare removes managed policy or supplies an OS sandbox. This MVP does not add Bedrock, Vertex, Foundry or arbitrary gateways.

The complete combination (subscription auth) was validated with a successful authenticated Execution on macOS with Claude Code 2.1.301; see the real-stream fixtures (`docs/research/fixtures/claude-code/` on branch `research/claude-code-cli-headless`). The API-key (`--bare`) mode and Windows remain unvalidated by a real run; the CI matrix and the manual smoke of [16](../../.scratch/surt-architecture/issues/16-test-strategy.md) cover them. Primary references: [CLI reference](https://code.claude.com/docs/en/cli-reference), [programmatic execution](https://code.claude.com/docs/en/headless), [bare mode](https://code.claude.com/docs/en/headless#start-faster-with-bare-mode).

## Runtime success and failure

Read the stream until the process ends and stdout/stderr have been drained. A `result` event is not necessarily the last line; do not publish outputs as soon as one success-shaped event arrives. Ignore unknown event types and fields for forward compatibility, retaining useful diagnostics. `system/init` is not always the first line (a `system/ui_invalidate` event preceded it in the captured runs). Invalid JSON on the protocol stream or ambiguous final results are protocol failures; they are not parsed from conversational text.

Acceptance requires all of the following:

1. No Interruption/Cancellation won the race with completion, and the process exited with code zero.
2. Initialization has the expected session ID, full model, compatible CLI version, permission mode and required tools. On mismatch, request a stop and never accept its output. `init` echoes the requested model even when it does not exist, so this check does not detect an unavailable model; and `plugins` stays populated under `--safe-mode`, so check `tools` (which also lists `StructuredOutput` when `--json-schema` is passed) and `mcp_servers`, not an empty plugin list.
3. The final authoritative result has `subtype = "success"`, `is_error = false`, no reported failure terminal reason, and a present `structured_output` object. `subtype` alone proves nothing: auth and model failures end with `subtype = "success"`, `is_error = true` and `terminal_reason = "api_error"`.
4. Surt independently validates that object against the generated envelope schema: exactly `outcome` and `outputs`, a permitted Outcome, exactly the declared output names and relative path strings.
5. Every required file, including every Item List item file, is a regular file within this Execution's delivery directory, valid for its format/schema and generic or registered built-in consistency checks. Reject traversal, absolute paths, drive/UNC prefixes, symbolic links, junctions and other reparse-point escapes in the path chain. Read and stage the validated bytes without reopening an attacker-controlled path after validation.

Only then may Surt publish the resulting Revisions and successful Outcome. Files left by a failed Execution are not accepted outputs. A present, schema-valid envelope is not evidence of delivery: with `--json-schema`, Claude Code pushes the model to call `StructuredOutput` even after a refusal or a denied write, and the captured model then declared a permitted Outcome with prose in place of a path; step 5 is the real gate. `--max-turns` counts turns, not tool calls, so an Execution stopped by it can leave every file written; SIGTERM ends the stream without a `result` (exit 143) and leaves partial files. The storage transaction and crash recovery around publication belong to sections H, D and I.

Auth failure, unavailable/mismatched model, unsupported settings, non-zero exit, missing result, malformed envelope, missing files, invalid Item List or a configured limit being reached are technical failures. `changes_requested` with valid feedback is a successful Outcome. Interruption and Cancellation retain their distinct meaning from section A; their state names, and the fact that neither counts toward the Attempt limit, are in [G. State Machines](G-state-machines.md).

The runtime neither invents billing precision nor reports a missing usage field as zero. Record available session/model/version identity, requested effort, turn counts, cost estimates and permission denials; unknown data remains unknown. The stream and stderr are written to `.surt/logs/<subject>/<execution-id>/` and survive cleanup; secret redaction is not in v1 ([I. Cleanup](I-cleanup.md#logs)).

## Agent Runtime and Fake Runtime

The runtime launches, follows and stops one Execution. It does not resolve workflow inputs, publish Revisions, record Approvals, retry Steps, select a next Step or write workflow state to SQLite.

Signature-level protocol sketch; workspace placement, error types and dispatch are in [B. Rust Workspace](B-workspace.md#traits):

```rust
pub trait AgentRuntime: Send + Sync {
    fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> impl Future<Output = Result<RuntimeCompletion, RuntimeError>> + Send;
}
```

- `RuntimeRequest` carries Execution/session identity, the Agent's frozen profile, prepared instructions/context, working and delivery directories, and fixed input handles. Secrets are resolved at launch and are not part of its persisted snapshot.
- `RuntimeEventSink` reports startup/process identity, progress, usage and diagnostics. The Driver records state; events do not bypass it.
- `StopSignal` requests an explicit stop. The runtime owns process-tree cleanup and uses section A's grace/forced-stop sequence. There is no wall-clock execution timer.
- `RuntimeCompletion` contains normalized initialization, exit/result evidence and usage for Surt's common acceptance path. Success is not a boolean shortcut around delivery validation.

The Fake Runtime takes the same request, creates delivery files and returns scripted events/results through the same protocol. It can simulate valid `done`, `approved` or `changes_requested` deliveries, missing/invalid JSON, missing/invalid files, model/init mismatch, crashes, configured-limit failures, unavailable usage and an Execution that waits until explicitly stopped. All deliveries pass through production validators; it never directly marks SQLite rows successful. Runtime stream parsing additionally requires the real fixtures, because a Fake Runtime cannot prove that the Claude adapter understands the live protocol.

## One Markdown document per Task

The Task Agent writes each Task's Markdown document itself. Surt does not generate a replacement document from the manifest's structured fields. The document contains the detailed implementation instructions for the later Builder; the structured fields provide the manifest's index, ordering and acceptance criteria. Human approval assesses their content together; structural validation does not establish natural-language consistency between them.

The manifest output declares one required item file named `task`:

```toml
[outputs.manifest]
format = "item_list"
json_schema = "schemas/manifest.schema.json"

[outputs.manifest.item_files.task]
format = "markdown"
```

Its final envelope still names only its declared output, `manifest`; item files are part of that output's Revision, not dynamic top-level outputs:

```json
{
  "outcome": "done",
  "outputs": {
    "manifest": "manifest.json"
  }
}
```

Example `manifest.json`, alongside the files in the delivery directory:

```json
[
  {
    "key": "A",
    "needs": [],
    "title": "Create users table",
    "description": "Add persistence for user accounts.",
    "acceptance_criteria": ["The migration creates the users table."],
    "files": { "task": "tasks/A.md" }
  },
  {
    "key": "B",
    "needs": ["A"],
    "title": "Implement account creation",
    "description": "Create a user account using the new persistence layer.",
    "acceptance_criteria": ["An account can be created and read back."],
    "files": { "task": "tasks/B.md" }
  }
]
```

The paths are portable relative paths rooted at the Execution's delivery directory, not at the JSON file's parent or a worktree. Use `/` as the separator; reject backslashes as well as absolute/drive/UNC paths to avoid platform-dependent interpretation. Keys and path components follow the portable naming rules of [A. Git Integration](A-git-integration.md#item-keys-and-item-file-paths) and are used verbatim, without encoding; a file path is not implicitly derived from an item key.

Delivery validates the Item List, every required `files.task` reference and every referenced non-empty UTF-8 `.md` file. A missing file, extra undeclared file name, wrong format or unsafe path fails the whole delivery; no partial manifest Revision becomes approvable. Unreferenced scratch files are not part of the published output.

One immutable Item List Revision contains the JSON and the referenced file bytes with their relative paths and content hashes. Its identity covers that complete set. Approval therefore covers every Task document, not only the JSON index. Editing any referenced `.md` creates a new Revision of the containing Item List and requires reapproval; it never changes an approved Revision in place. Edits are detected by hashing the Working Copy and validated with these same delivery rules; see [H. Artifact Storage, Freezing and Revalidation](H-artifact-storage.md).

On expansion, a Child Workflow Run pins its item and files to that approved Item List Revision. The Context Assembler supplies a read-only local binding of the item's `task` file as `task.md` in the Builder's prepared input context. Reviewer receives the same pinned item file. The child does not receive other items' files, and materializing these input views does not create new Artifact Revisions or change their origin. Any Surt-managed change is subject to the revalidation rules of [section H](H-artifact-storage.md#revalidation); the read-only binding is not a promise of OS-level confinement for a Builder with shell access.

The engine knows a generic optional `files` map on Item Lists and generic `item_files` format declarations. It does not know what a Task, Builder or `task.md` means. The built-in workflow's instructions give that attachment its role. See [E. Workflow Definition Schema](E-workflow-schema.md) and [ADR 0004](../adr/0004-item-list-files-share-approved-revision.md).
