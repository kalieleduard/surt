# Definitions live in the project

Every Workflow Definition and Agent a run uses is a file under `.surt/workflows/` and `.surt/agents/`. The engine reads them when a root run is created and freezes them; nothing is compiled into the binary or merged over defaults. The binary keeps the feature workflow only as a **Template** that `surt init --template feature` writes once into `.surt/`. The CLI is generic over whatever workflows the project defines: `surt run <workflow>` and `surt continue <subject>` replace the phases `feature`, `plan` and `start`.

## Considered Options

- **Built-in definitions with field-level overrides (M4):** upgrades reach every project automatically, but a project cannot change or add a workflow, cannot read its effective Agents without the binary's sources, and the CLI has to know the built-in workflow's phases and artifacts.
- **Built-ins plus project workflows side by side:** keeps automatic upgrades for the feature workflow, but two sources of truth for the same names and a CLI that still special-cases the built-in one.
- **Everything in the project, written from a Template (chosen):** one source of truth, the CLI and the engine know no workflow, and a team edits its flow like any other code.

## Consequences

- An upgrade of Surt no longer changes what an existing project runs. Improvements to the Template reach a project only when it writes the files again (a fresh project, or a deleted file); `surt init` never overwrites.
- Delivery checks are declared on Agent outputs and frozen with the Agent, instead of being registered by the binary.
- A workflow declares its run inputs (`[inputs]`); the CLI supplies them with `--input`. The engine's Operations keep their artifact contract (`base`, `item`, `commit`, `feedback`, `check-report`, `conflict`, `integration`), which a workflow must honour.
- This amends [ADR 0011](0011-two-crates-keep-the-engine-generic.md): the crates and their dependency direction are unchanged, but the binary no longer holds the feature workflow as a `Catalog`, only as Template bytes.
