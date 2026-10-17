# Estrutura do workspace Rust e traits centrais

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 05, 06, 07, 11

## Question

Como o código se organiza em crates e módulos, e quais são as interfaces (traits) entre os componentes?

Decidir: quantos crates (ex.: `surt-core` + `surt-cli`?) e por quê; módulos do núcleo (engine, scheduler, runtime, context, artifacts, store, recovery, lifecycle, git); traits e suas assinaturas (`AgentRuntime`, `StateStore`, `GitOps`, `Clock`, etc.) — só onde há mais de uma implementação real ou de teste; fronteiras de erro (`thiserror` × `anyhow`); dependências justificadas uma a uma; onde ficam workflows/agentes embutidos (`include_str!`).

Saída: seções A (com diagrama Mermaid) e B. Usar `/codebase-design`.

## Context

De [Contrato de agente](07-contrato-de-agente.md): [F. Agent Contract](../../../docs/architecture/F-agent-contract.md#agent-runtime-and-fake-runtime) fixa as responsabilidades e um esboço de `AgentRuntime::execute(request, events, stop)`, sem corpos. Este ticket decide representação/dispatch, tipos concretos, fronteiras de erro e localização nos crates. Fake Runtime usa o mesmo caminho de validação e não altera SQLite diretamente. Item files são genéricos no núcleo; schemas e consistência de review são do workflow embutido.

De [Schema TOML de workflow](06-schema-toml-de-workflow.md): o parser/validador de definições é um módulo do núcleo — cinco `kind` (`agent`, `approval`, `operation`, `loop`, `per_item`), `deny_unknown_fields`, `toml` com `preserve_order`, 8 checagens estáticas que precisam carregar `agent.toml` e o registro de Operations. O engine resolve inputs por nome, avalia `when`/`until` por igualdade de Outcome e expande Item Lists.

De [Modelo de integração Git](09-modelo-de-integracao-git.md) ([A. Git Integration](../../../docs/architecture/A-git-integration.md)): módulo/crate de Git que só fala com a CLI `git` (argv, sem shell, sem libgit2/gix) e expõe as seis Operations do registro (`git.create_integration_branch`, `git.create_task_worktree`, `checks.run`, `git.integrate`, `git.snapshot_integration`, `git.merge_into_base`) por trás de uma interface de registro de Operations do engine (id → Inputs, Outcomes declarados, artifacts produzidos). A validação de evidência do `commit` do Builder é uma consistency check registrada pelo workflow embutido, não um branch do engine por id de agente. O engine ganha "initial artifacts" fornecidos pelo comando criador (generalização do `request`). As regras de nome de key/path são validação genérica de Item List.

De [Máquina de estados do loop Builder ↔ Reviewer](10-loop-builder-reviewer.md) ([G. State Machines](../../../docs/architecture/G-state-machines.md)): um único Agent Profile por Agent (`[profile]` no `agent.toml`, sem seleção nem escalada). O Agent Runtime e as Operations precisam classificar uma falha como retentável ou não (não-retentável bloqueia na 1ª); o tipo de erro deve carregar essa classe. A derivação do estado exibido do run (a partir dos Step Runs) é lógica do núcleo, não da CLI.
