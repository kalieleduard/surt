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
