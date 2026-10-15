# Modelo de processo e controle de concorrência

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 01, 18

## Question

Quem executa o workflow, por quanto tempo, e como o Surt impede trabalho duplicado entre processos?

Decidir: quais comandos "dirigem" o run (ex.: `surt feature` roda até `AWAITING_APPROVAL` e sai; `surt approve` continua em foreground?), o que acontece com Ctrl-C (cancelar execuções filhas ou deixá-las órfãs para reconciliação?), lock por workflow run (arquivo × lease no SQLite × ambos), heartbeat/lease das executions e prazo de expiração, várias features rodando em paralelo no mesmo repo (cada uma em seu processo?), limite global de concorrência de agentes por projeto entre processos distintos, como o processo filho (Claude Code) é supervisionado (process group/job object), e o que no design garante que um daemon futuro caiba sem retrabalho.
