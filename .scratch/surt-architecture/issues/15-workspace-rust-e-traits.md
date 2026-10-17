# Estrutura do workspace Rust e traits centrais

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Assignee: Kaliel
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
- **Cleanup:** passos idempotentes após o estado terminal, sob o lock do run; `surt delete` = Git/disco e depois o purge numa transação ([I. Cleanup](../../../docs/architecture/I-cleanup.md)). Mora no núcleo (genérico: worktrees, refs, diretórios), não na CLI.

## Answer

Resolvido por troca humana em duas rodadas (12 perguntas, todas aceitas com a recomendação). Seção completa em [B. Rust Workspace, Modules and Traits](../../../docs/architecture/B-workspace.md); diagrama de componentes e fluxo de um comando em [A. Overview](../../../docs/architecture/A-overview.md). D e F passam a apontar para B. ADRs novos: [0011 — Two crates keep the engine generic](../../../docs/adr/0011-two-crates-keep-the-engine-generic.md) e [0012 — The Agent Runtime is the only trait](../../../docs/adr/0012-agent-runtime-is-the-only-trait.md).

Decisões finais:

- **Crates:** workspace com `surt-engine` (lib genérica) e `surt` (bin: clap, fases, textos, exit codes, workflows e agentes embutidos). O compilador impede o motor de importar a CLI; testes do motor usam workflows sintéticos porque os reais não existem no crate.
- **Fachada:** a CLI só vê `Project` (`open`, `find_subject`, `create_run`, `drive`, `approve`, `reject`, `request_stop`, `cancel`, `restore`, `delete`, `status`, `inspect`, `show`). `drive` = `try_lock` → reconcile → avançar → cleanup. Precondições de fase e confirmações ficam na CLI; resolução de Subject (nome/slug/prefixo) no motor.
- **Módulos do motor:** `project`, `workflow`, `drive`, `reconcile`, `artifacts`, `context`, `runtime`, `operations`, `git`, `cleanup`, `store`, `platform`; só `project` e os tipos da fachada são públicos.
- **Operations:** registro fechado (`enum` + `match`) no motor, sem trait. **Run Branch** passa a ser a branch de qualquer Workflow Run (root ou filho), alinhado à tabela `run_branch`; Integration Branch e Task Branch são os nomes do workflow `feature`. Ids renomeados: `git.create_run_branch`, `git.create_child_worktree`, `git.snapshot_run_branch` (A-git, E, D, I, `integration.schema.json`; tickets fechados não foram reescritos).
- **Traits:** só `AgentRuntime` (Claude Code + Fake), `async fn` nativo com futuro `Send`, despacho por `&impl AgentRuntime` no `drive`. Sem trait para store, Git, Operations ou relógio: SQLite e `git` reais em tempdir, `tokio::time::pause()`, cálculos com `now` como argumento.
- **Store:** um método por checkpoint transacional de I (cada um um `BEGIN IMMEDIATE` com seu `event`), nenhuma transação exposta; `drive` relê o `RunGraph` a cada tick.
- **Scheduler:** função pura `decide(&RunGraph, &Ctx) -> Vec<Action>`; as regras de G viram testes de tabela sem I/O.
- **Erros:** `thiserror` no motor; `Error = Refused(Refusal) | Internal`; `Refusal` é dado sem texto (a CLI escreve `✖ … / fix: …`); `DriveOutcome = AtGate | Succeeded | Blocked | Cancelled | Interrupted | Halted(Refusal)` mapeia para 0/0/4/5/130/3. `FailureClass` vive no `RuntimeError` do adaptador e é gravada na Execution, nunca cruza a fachada. `anyhow` só na CLI.
- **Progresso e stop:** o motor nunca imprime; emite `Progress` tipado por `mpsc`. Ctrl-C (graceful/force) e o pedido de `surt stop` no SQLite chegam pelo mesmo `StopSignal` (`watch<StopLevel>`).
- **Embutidos:** `Catalog` montado no crate `surt` com `include_str!`, passado a `Project::open`; o motor faz o merge de `.surt/agents/`. O motor embute só migrations e os schemas de artefatos de Operations (`base`, `check-report`, `conflict`, `integration`).
- **Toolchain e dependências:** edition 2024, `rust-version = "1.94"`, stable fixada, Tokio multi-thread; `sqlx` 0.9, `serde`/`serde_json`/`toml`, `jsonschema` (sem HTTP), `uuid` v7, `sha2`, `jiff`, `process-wrap` 10, `libc`/`windows-sys`, `dunce`, `thiserror`, `tracing`; na CLI `clap`, `anyhow`, `tracing-subscriber`, `anstream`/`anstyle`; `tempfile` em dev. Fora: `dirs` (fato verificado: `std::env::home_dir` sem depreciação no rustc 1.99), `indicatif`/`crossterm`, `async-trait`, `git2`/`gix`, `chrono`/`time`, `include_dir`.

`CONTEXT.md`: novo termo **Run Branch**; Integration Branch e Task Branch redefinidos como Run Branches do workflow `feature`. Notas de contexto acrescentadas a [Estratégia de testes](16-estrategia-de-testes.md) e [Roadmap e Milestone 1](17-roadmap-e-milestone-1.md). Nenhum ticket novo.

Revisão contra `rules/` (2026-10-17): `drive` depender de `Store` e `git` concretos contraria o DIP de `rules/architecture.md`; mantido como exceção registrada no [ADR 0012](../../../docs/adr/0012-agent-runtime-is-the-only-trait.md), com a condição de que `RunGraph`, `Action`, `Ctx` e os enums de estado não importem `sqlx` nem `git` e o `store` mapeie linhas para eles ([B](../../../docs/architecture/B-workspace.md#the-scheduler-is-a-pure-function)). `jiff` passa a servir timestamps em segundos Unix.
