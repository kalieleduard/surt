# A. Overview

Part of section A (Architecture): the parts of Surt and how they depend on each other. The other parts of section A are [Process Model and Concurrency](A-process-model.md) and [Git Integration](A-git-integration.md). Crates, module contracts and traits are specified in [B. Rust Workspace, Modules and Traits](B-workspace.md). Vocabulary is defined in [`CONTEXT.md`](../../CONTEXT.md).

## Components

```mermaid
flowchart TD
  CLI["<b>surt</b> (bin)<br/>clap · run/continue · messages · exit codes<br/>feature Template: bytes for surt init"] --> P

  subgraph engine["<b>surt-engine</b> (lib, generic)"]
    P["project<br/>facade Project · finds .surt/ · configuration"]
    P --> W["workflow<br/>read .surt/ · parse TOML · validate · freeze + SHA-256"]
    P --> D["drive<br/>tick loop · decide() · limits · pause"]
    P --> CL["cleanup<br/>after terminal · delete"]
    D --> R["reconcile<br/>Intentions × Git and processes"]
    D --> A["artifacts<br/>Revisions · delivery validation · Working Copy"]
    D --> CX["context<br/>Context Assembler"]
    D --> RT["runtime<br/>AgentRuntime · Claude Code · Fake"]
    D --> O["operations<br/>closed registry of 6"]
    O --> G["git<br/>argv · git.lock"]
    R --> G
    CL --> G
    P --> S
    D --> S
    R --> S
    CL --> S
    A --> S["store<br/>SQLite · one method per checkpoint"]
    D --> PL
    R --> PL
    RT --> PL
    CL --> PL["platform<br/>locks · process identity · safe removal"]
  end

  RT -. spawns .-> CC(["claude CLI"])
  G -. argv .-> GIT(["git CLI"])
  S -. file .-> DB[(".surt/surt.db")]
```

- Arrows are compile-time dependencies. Only `project` is visible to the CLI.
- `runtime` is the only module behind a trait ([B](B-workspace.md#traits)). `git`, `store` and `platform` talk to the real tool, database and OS, in production and in tests.
- Nothing in `surt-engine` knows the feature workflow, and neither does the CLI. Every Workflow Definition and Agent is a project file under `.surt/workflows/` and `.surt/agents/`, read and frozen when a root run is created; the CLI only holds the feature Template's bytes, which `Project::init` writes on `surt init --template feature` ([ADR 0013](../adr/0013-definitions-live-in-the-project.md)).

## One driving command, end to end

```mermaid
sequenceDiagram
  participant U as user
  participant C as surt (CLI)
  participant P as Project::drive
  participant S as store
  participant RT as AgentRuntime
  U->>C: surt continue add-login
  C->>P: open → resolve_subject → active run (no gate precondition)
  C->>P: drive(run, &ClaudeCodeRuntime, progress, stop)
  P->>P: try_lock run · reconcile
  loop every tick (~1 s)
    P->>S: load_run_graph
    P->>P: decide(graph, ctx) → actions
    P->>RT: execute(request, events, stop)
    RT-->>P: RuntimeCompletion | RuntimeError(class)
    P->>S: accept_execution / end_execution (+ event)
    P-->>C: Progress (rendered as lines + footer)
  end
  P->>P: cleanup (if terminal)
  P-->>C: DriveOutcome
  C-->>U: summary + next command, exit code
```
