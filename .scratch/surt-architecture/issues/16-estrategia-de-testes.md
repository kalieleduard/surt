# Estratégia de testes

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 07, 10, 12, 19

## Question

Como se testa o Surt sem gastar tokens, incluindo crashes em pontos críticos?

Decidir: desenho do Fake Agent Runtime (roteiros por agente, outputs determinísticos, falhas injetáveis, latência); como simular crash entre intenção e efeito (fault points? matar o processo de verdade em teste de integração?); repositório Git temporário por teste; testes de SQLite (migrations, constraints); o E2E do brief (feature → spec → 2+ tasks → aprovação → builders → reprovação → novo builder → aprovação → shipper → cleanup → verificação do estado) e onde ele roda; testes multiplataforma (CI em macOS/Linux/Windows).

## Context

De [Modelo de processo e controle de concorrência](05-modelo-de-processo-e-concorrencia.md): testar entre processos reais — dois Drivers no mesmo run (o segundo recusa), `kill -9` liberando run lock e vagas, órfão Unix sobrevivendo ao Driver e sendo morto no próximo `start`, Ctrl-C duplo, migrations concorrentes sob `db.lock`, e a escalada SIGTERM/`CTRL_BREAK_EVENT` → kill no Windows.
