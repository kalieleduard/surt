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

De [Schema TOML de workflow](06-schema-toml-de-workflow.md): o parser/validador de definições é um módulo do núcleo — cinco `kind` (`agent`, `approval`, `operation`, `loop`, `per_item`), `deny_unknown_fields`, `toml` com `preserve_order`, 8 checagens estáticas que precisam carregar `agent.toml` e o registro de Operations. O engine resolve inputs por nome, avalia `when`/`until` por igualdade de Outcome e expande Item Lists.
