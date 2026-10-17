# Estrutura do workspace Rust e traits centrais

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 05, 06, 07, 11

## Question

Como o código se organiza em crates e módulos, e quais traits existem entre eles?

Decidir, só para a v1:
- **Crates:** um só ou `surt-core` + `surt-cli`? Justificar pela fronteira real (a CLI é a única que conhece fases do workflow `feature`).
- **Módulos do núcleo:** engine, store, runtime, git/operations, artifacts, recovery, cleanup — o que cada um esconde.
- **Traits** só onde há duas implementações de verdade: `AgentRuntime` (Claude Code + Fake), e talvez `Clock`. `StateStore`/`GitOps` só se o teste realmente precisar trocar.
- **Erros:** `thiserror` no núcleo × `anyhow` na CLI, e como o erro carrega a classe da falha (`transient` / `not_retryable` / `retryable`) e vira exit code.
- **Dependências:** lista curta do que entra no M1.
- **Embutidos:** workflows `feature`/`task-cycle` e agentes via `include_str!`.

Saída: seções A (diagrama Mermaid) e B. Usar `/codebase-design`.

## Context

- **AgentRuntime:** esboço `execute(request, events, stop)` em [F](../../../docs/architecture/F-agent-contract.md#agent-runtime-and-fake-runtime); o Fake Runtime passa pela mesma validação e não escreve no SQLite.
- **Workflow:** parser/validador é do núcleo — cinco `kind`, `deny_unknown_fields`, checagens estáticas que leem `agent.toml` e o registro de Operations ([E](../../../docs/architecture/E-workflow-schema.md)).
- **Operations:** registro id → Inputs, Outcomes, artifacts; as seis do Git falam só com a CLI `git` (argv, sem shell) ([A](../../../docs/architecture/A-git-integration.md)).
- **Store:** `sqlx::query_as` + `FromRow` em runtime, `sqlx::migrate!` sob `db.lock`, escrita sempre `BEGIN IMMEDIATE`, todo estado grava seu `event` na mesma transação; migration de referência [`0001_initial.sql`](../../../docs/architecture/migrations/0001_initial.sql) ([D](../../../docs/architecture/D-database.md)).
- **CLI:** só ela conhece fases, precondições de `plan`/`start` e a tabela gate → próximo comando; leitura não pega lock nem reconcilia; exit codes 0/1/2/3/4/5/130 ([J](../../../docs/architecture/J-cli.md)).
