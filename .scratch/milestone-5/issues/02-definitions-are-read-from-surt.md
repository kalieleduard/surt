# 02: Definitions are read from `.surt/`, written by `surt init --template feature`

Spec: [Milestone 5](../spec.md) (stories 1–11) · Plan: [E](../../../docs/architecture/E-workflow-schema.md), [F](../../../docs/architecture/F-agent-contract.md), [B](../../../docs/architecture/B-workspace.md), [ADR 0013](../../../docs/adr/0013-definitions-live-in-the-project.md)

**What to build:** the engine reads the workflow closure and its Agents from `.surt/workflows/` and `.surt/agents/` when a root run is created; the binary no longer passes a `Catalog`. `surt init --template feature` writes today's built-in files into `.surt/` (never overwriting), and plain `surt init` writes none. `surt feature`, `plan` and `start` keep working, now on the project's files.

```
$ surt init --template feature
✔ initialized .surt/
  .surt/workflows/feature.toml             created
  .surt/agents/reviewer/instructions.md    created
  …
$ surt init --template feature
  .surt/agents/reviewer/instructions.md    kept
```

**Blocked by:** 01

**Status:** resolved

- [x] `Project::open(cwd)` takes no catalog; `create_run` reads `.surt/workflows/<name>.toml`, its `per_item` children transitively and the Agents they reference, and nothing else; the file stem must equal `name`; `agent.toml` and `instructions.md` are required, `schemas/**` optional, `.gitkeep`/`.keep` ignored
- [x] Every M1–M4 check applies to every file read (workflow schema and static validation, full `agent.toml` schema, profile checks, schema presence, local `$ref`s, schemas compile, `fake` only with `test-support`, UTF-8, CRLF-independent identity); failures are `InvalidDefinition` with the `.surt/` path, field or step and reason, exit `3`, no row written
- [x] `UnknownWorkflow` lists the workflows in `.surt/workflows/`, or suggests `surt init --template feature` when there are none; the "invalid built-in definition … reinstall surt" wording is gone
- [x] The M4 merge is removed: `with_local`, effective TOMLs, field merge, `blame_overrides`, `UnknownAgentDirectory`, the `local agents:` creation line and their tests; `Catalog` becomes the engine's internal owned form of what was read (or is removed)
- [x] `Project::init` takes Template files as data and reports each file as created or kept; `.gitignore` versions `config.toml`, `workflows/**` and `agents/**`; the engine knows no Template name
- [x] The binary moves `assets/{workflows,agents}` to `assets/templates/feature/` and embeds them for `surt init --template <name>`; an unknown template refuses with the list; the engine's operation schema constants are written into the Agents' `schemas/` exactly where today's catalog listed them
- [x] `builtin/tests.rs` becomes the Template's tests, validating the written files through the engine
- [x] Engine test fixtures write their synthetic definitions into the test project's `.surt/`; CLI `sandbox.init()` runs `surt init --template feature`; `init.rs` and `overrides.rs` CLI tests are rewritten for whole files
- [x] `Project` tests: a run frozen from disk ignores later edits; an unrelated broken Agent does not refuse; a broken referenced one does with path and field; stem ≠ `name` refuses
- [x] B, E, F, H and J describe the files, the Template and `init --template`

## Comments

- Resolved on `m5/02-definitions-from-surt`. The engine reads the closure through a `DefinitionSource` port (`workflow/files.rs`) backed by `project/definitions.rs`; `create_run` first checks the root workflow file exists (`Refusal::UnknownWorkflow { name, known }`), then freezes. `project::check_workflow(root, workflow, initial_artifacts)` validates the files under `<root>/.surt/` without a database; the Template tests use it.
- `InvalidDefinition` now carries `field` and its `DefinitionFile` gains `AgentFile { agent, path }` and `path()`; `InvalidAgentOverride`, `InvalidOverride`, `StepUse`, `WorkflowStep`, `LocalFiles` and `Problem::UnknownAgentDirectory` are gone; `Problem::MissingFile` is new (`fix: create <file>`).
- `runtime = "fake"` is accepted under `cfg!(any(test, feature = "test-support"))`, so engine unit tests keep using fake Agents without the feature; the release refusal is tested in the binary (`template/tests/runtime.rs`, run by `cargo test -p surt` without `test-support`).
- `Project::init(cwd, &[(path, bytes)])`; `Initialized.files` is a `Vec<(InitFile, FileState)>` with `InitFile::Template(path)`; `was_already_initialized` means the database existed. The init lines are `<path>  created|kept`; the `next:` line is unchanged (ticket 03 changes it).
- For 03/04: `UnknownWorkflow` with known workflows says `fix: add .surt/workflows/<name>.toml`; the CLI still passes the `request`/`base` initial artifacts.
