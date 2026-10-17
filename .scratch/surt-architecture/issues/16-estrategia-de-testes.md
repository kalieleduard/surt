# Estratégia de testes

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Blocked by: 07, 10, 12
Assignee: Kaliel

## Question

Como se testa o Surt sem gastar tokens, com confiança suficiente para entregar o M1?

Decidir, só para a v1:
- **Fake Agent Runtime:** como se escreve o roteiro de cada agente (entrega válida, entrega inválida, review reprovado, falha) e como o teste escolhe o roteiro.
- **Queda entre intenção e efeito:** um mecanismo só — fault points no código ou matar o processo de verdade — e quais checkpoints de [I](../../../docs/architecture/I-recovery.md) são cobertos no M1.
- **Git:** repositório temporário real por teste; cenários mínimos: merge limpo, conflito, Checks falhando.
- **E2E do brief:** feature → spec → aprovação → 2+ tasks → builders → review reprovado → novo builder → aprovação → shipper → cleanup → verificar estado final. Onde roda e quanto tempo pode levar.
- **CI:** matriz macOS/Linux/Windows — o que roda em cada um.

Fora da v1 (não decidir aqui): teste de toda linha das tabelas de recuperação, Ctrl-C duplo, migrations concorrentes, matriz de versões do Git, símbolos ASCII sem UTF-8, snapshot de todas as mensagens, virada de data no slug.

## Context

- **Loop Builder ↔ Reviewer:** comportamento principal a provar com o Fake — 3 entregas inválidas → `blocked`, rodar de novo renova o orçamento; review reprovado abre nova Iteration com `feedback`; Task bloqueada segura só seus dependentes ([G](../../../docs/architecture/G-state-machines.md)).
- **Concorrência:** segundo Driver no mesmo run recusa; `kill -9` libera locks ([A-process-model](../../../docs/architecture/A-process-model.md)).
- **SQLite:** testes contra arquivo real (não in-memory); os CHECKs/índices únicos que sustentam invariantes (≤1 root ativo por Subject, ≤1 Execution viva por Step Run) ([D](../../../docs/architecture/D-database.md)).
- **CLI:** cada exit code (0, 2, 3, 4, 5, 130) e leitura não altera nada ([J](../../../docs/architecture/J-cli.md)).
- **Cleanup:** o E2E termina verificando o estado final de [I. Cleanup](../../../docs/architecture/I-cleanup.md#what-each-ending-removes) — worktrees e Task Branches fora, Integration Branch e `.surt/logs/<subject>/` dentro, `cleaned_at` gravado; falha de remoção deixa `cleanup pending` e o próximo comando termina; `surt delete` interrompido termina ao rodar de novo.
- **Fixture real:** quando [19](19-fixture-stream-real-claude-code.md) sair, o stream capturado vira teste do parser do Claude Code Runtime; não bloqueia o desenho do Fake.
- **Seams de teste (de [B](../../../docs/architecture/B-workspace.md)):** só `AgentRuntime` tem trait; `FakeRuntime` é público em `surt-engine::runtime`, e falta decidir como o binário `surt` o seleciona num E2E. Store e Git são reais em tempdir; timers com `tokio::time::pause()`. As regras de G se testam na função pura `decide(&RunGraph, &Ctx)`. Testes do motor usam workflows sintéticos (o `feature.toml` não existe no crate do motor).

## Answer

Resolvido em sessão de grilling (três rodadas, todas aceitas). Nenhum artefato novo em `docs/architecture/`: a decisão é de processo de teste, não de contrato, e o `B-workspace.md` já aponta para este ticket.

Decisões:

- **Roteiro do Fake (Q1):** testes do motor usam roteiro tipado em Rust, indexado por chave lógica do step + Iteration + Attempt (`Script::new().on("build", 1, 1, Behavior::Commit)`). Arquivo TOML por cenário só quando o binário `surt` precisar receber o roteiro de fora. Comportamentos: os da [F. Agent Contract](../../docs/architecture/F-agent-contract.md#agent-runtime-and-fake-runtime).
- **Queda entre intenção e efeito (Q2):** fault points no store, cobrindo os oito checkpoints de [I. Recovery](../../docs/architecture/I-recovery.md#transactional-checkpoints). Um smoke de `kill` real complementa (Q8). Nenhum outro mecanismo no M1.
- **Git (Q3):** repositório temporário real por teste; só os três cenários do ticket (merge limpo, conflito, Checks falhando). Estados parciais do Git são cobertos injetando falha nos checkpoints, sem repositórios novos.
- **CI (Q4):** toda a suíte roda nos três SOs com Fake Runtime e Git real (≥ 2.38, pinado por runner). Claude Code real fica fora da CI e vira smoke manual com o fixture do [19](19-fixture-stream-real-claude-code.md). Não existe `.github/workflows` hoje; a CI é entregável do M1. Antes dos testes, cada SO roda `cargo fmt --all -- --check` e `cargo clippy --all-targets --features test-support -- -D warnings` (acrescentado na revisão contra `rules/rust.md`, 2026-10-17).
- **E2E (Q5):** roda via binário `surt`, cobrindo os exit codes 0/2/3/4/5/130 junto com o fluxo do brief.
- **Seleção do Fake (Q6):** um cargo feature `test-support`, desligado por padrão, gateia a seleção do Fake por `SURT_RUNTIME` (com caminho do roteiro) e os fault points. Release nunca o habilita; CI e `cargo test` o ativam.
- **Orçamento (Q7):** E2E com teto de 60 s por SO, sem `sleep` real. Estouro vira sinal para cortar cenário.
- **Smoke de `kill` (Q8):** `kill` durante uma Execution em espera do Fake (estado `running`, determinístico). A retomada deve terminar o run, com a Execution morta marcada `interrupted` e novo Attempt, conforme [G](../../docs/architecture/G-state-machines.md).

Mantidos como fora da v1 (sem mudança): teste de toda linha das tabelas de recuperação, Ctrl-C duplo, migrations concorrentes, matriz de versões do Git, símbolos ASCII sem UTF-8, snapshot de todas as mensagens, virada de data no slug.
