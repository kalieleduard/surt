# Modelo de processo e controle de concorrência

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 01, 18

## Question

Quem executa o workflow, por quanto tempo, e como o Surt impede trabalho duplicado entre processos?

Decidir: quais comandos "dirigem" o run (ex.: `surt feature` roda até `AWAITING_APPROVAL` e sai; `surt approve` continua em foreground?), o que acontece com Ctrl-C (cancelar execuções filhas ou deixá-las órfãs para reconciliação?), lock por workflow run (arquivo × lease no SQLite × ambos), heartbeat/lease das executions e prazo de expiração, várias features rodando em paralelo no mesmo repo (cada uma em seu processo?), limite global de concorrência de agentes por projeto entre processos distintos, como o processo filho (Claude Code) é supervisionado (process group/job object), e o que no design garante que um daemon futuro caiba sem retrabalho.

## Context

Surgiu de [Primitivas multiplataforma](18-primitivas-multiplataforma.md): kill de árvore exige process group (Unix) / Job Object (Windows) — adotar `process-wrap` ou implementar?; escalonamento gracioso (SIGTERM → SIGKILL; Ctrl+Break no Windows) e prazos; lock por workflow em arquivo separado via `std::fs::File::lock` × lease no SQLite; identidade de PID = start time nativo + boot id.

Também de [SQLx + SQLite na prática](03-sqlx-sqlite-na-pratica.md): um writer por vez entre processos; transações de escrita devem ser `BEGIN IMMEDIATE` com retry em BUSY; migração concorrente entre processos não é protegida pelo sqlx — serializar (ex.: sob o lock de arquivo) na abertura do DB.
