# Two crates keep the engine generic

Amended by [ADR 0013](0013-definitions-live-in-the-project.md) (Milestone 5): the binary no longer holds the feature workflow as definitions passed to the engine as a `Catalog`; it holds only the feature Template's bytes, which `surt init --template feature` writes into `.surt/`, and the engine reads every definition from there. The two crates and their dependency direction are unchanged.

Surt is a Cargo workspace with two crates: `surt-engine`, a library that knows only engine concepts (Workflow Run, Step Run, Execution, Artifact, Operation…), and `surt`, the binary that holds the CLI and the built-in feature workflow (its TOML definitions, its five Agents, its phases, its messages). The engine cannot import anything from the binary, and the feature workflow's files do not exist inside the engine crate.

## Considered Options

- **One package with a library and a binary, split by modules:** cheapest to set up, but the rule "the core does not know Spec, Task, Builder, Reviewer or Shipper" is then only a convention. Importing a CLI `Phase` enum into the scheduler, or loading the embedded `feature.toml` in an engine test, compiles.
- **Two crates (chosen):** the compiler enforces the dependency direction. Engine tests must write their own synthetic workflows because the real ones are not reachable, which keeps proving that the core is generic.
- **More crates (engine, Claude adapter, Git, CLI):** splits without a second consumer of any piece. Each extra crate adds `pub` surface and versioning with no boundary that matters today.

## Consequences

- Everything the CLI uses from the engine must be `pub` in `surt-engine`; everything else stays `pub(crate)`. The engine's interface is one facade type, `Project`, plus the types it returns ([B](../architecture/B-workspace.md#the-engines-interface-project)).
- The built-in definitions reach the engine as data (a `Catalog` built with `include_str!` in `surt`). The engine only embeds what it owns: its migrations and the schemas of Artifacts produced by Operations.
- Splitting the engine further later is cheap; merging the crates back would remove the guarantee silently.
