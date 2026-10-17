# Estratégia de testes

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 07, 10, 12

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
- **Fixture real:** quando [19](19-fixture-stream-real-claude-code.md) sair, o stream capturado vira teste do parser do Claude Code Runtime; não bloqueia o desenho do Fake.
