# Modelo de processo e controle de concorrência

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Blocked by: 01, 18
Assignee: Kaliel

## Question

Quem executa o workflow, por quanto tempo, e como o Surt impede trabalho duplicado entre processos?

Decidir: quais comandos "dirigem" o run (ex.: `surt feature` roda até `AWAITING_APPROVAL` e sai; `surt approve` continua em foreground?), o que acontece com Ctrl-C (cancelar execuções filhas ou deixá-las órfãs para reconciliação?), lock por workflow run (arquivo × lease no SQLite × ambos), heartbeat/lease das executions e prazo de expiração, várias features rodando em paralelo no mesmo repo (cada uma em seu processo?), limite global de concorrência de agentes por projeto entre processos distintos, como o processo filho (Claude Code) é supervisionado (process group/job object), e o que no design garante que um daemon futuro caiba sem retrabalho.

## Context

**Requisito do usuário** (de [Glossário do domínio](01-glossario-do-dominio.md)): deve ser possível deixar N features aprovadas e rodá-las **simultaneamente no mesmo repo**. O modelo de domínio já permite (a invariante é ≤1 Workflow Run ativo por Subject, não por projeto); falta decidir aqui como os processos se organizam e qual limite global de agentes vale entre eles.

Surgiu de [Primitivas multiplataforma](18-primitivas-multiplataforma.md): kill de árvore exige process group (Unix) / Job Object (Windows) — adotar `process-wrap` ou implementar?; escalonamento gracioso (SIGTERM → SIGKILL; Ctrl+Break no Windows) e prazos; lock por workflow em arquivo separado via `std::fs::File::lock` × lease no SQLite; identidade de PID = start time nativo + boot id.

Também de [SQLx + SQLite na prática](03-sqlx-sqlite-na-pratica.md): um writer por vez entre processos; transações de escrita devem ser `BEGIN IMMEDIATE` com retry em BUSY; migração concorrente entre processos não é protegida pelo sqlx — serializar (ex.: sob o lock de arquivo) na abertura do DB.

## Answer

Resolvido em sessão de grilling. Documento: `docs/architecture/A-process-model.md`; trade-off em `docs/adr/0002-file-locks-over-sqlite-leases.md`; termos **Driver**, **Interruption** e **Cancellation** em `CONTEXT.md`.

Decisões:
- **Quem dirige:** o núcleo tem uma primitiva só, `drive(root_run_id)`, que avança o root run e seus Child Runs até só restar espera por humano ou o fim. No workflow de feature, cada comando é dono de uma fase: `surt feature` (até o gate da spec), `surt plan` (Task Agent, até o gate das tasks; exige spec aprovada), `surt start` (builders → reviews → Shipper, até a entrega final; exige tasks aprovadas). **`surt approve` só registra a Approval, nunca dirige.** Rodar o mesmo comando de novo continua de onde parou; sem `surt resume` no M1.
- **Unidade de propriedade:** um processo dirige um root run com todos os seus Child Runs; N features = N processos.
- **Um Driver por run:** lock de arquivo exclusivo (`.surt/runtime/locks/<root-run-id>.lock`, `File::lock`) decide; o SQLite guarda pid + start time + boot id só para exibição e recuperação. Sem lease, heartbeat ou expiração por tempo (sono do notebook não gera dono duplicado). Execution `RUNNING` com o lock do run livre = órfã.
- **Ctrl-C = Interruption:** parada graciosa dos agentes em paralelo, Executions marcadas como interrompidas, run retomável com novos Attempts; Native Operation em andamento nunca é cortada pelo primeiro Ctrl-C; o segundo força. SIGHUP/fechar console = mesmo caminho. **Cancellation** só via `surt cancel`.
- **Controle externo só pelo SQLite:** `cancel` grava um cancel request que o Driver lê a cada tick (~1s); sem Driver vivo, o próprio `cancel` pega o lock e cancela. Nunca por sinais entre processos Surt.
- **Limite de agentes por projeto** (`max_concurrent_agents`, default 3), porque o Surt roda em vários projetos na mesma máquina; só Executions de agente contam. Mecanismo: vagas = arquivos de lock `.surt/runtime/slots/agent-<i>.lock` com `try_lock` e polling de ~500ms; sem justiça entre processos no M1.
- **Git nativo serializado por repo** sob `.surt/runtime/locks/git.lock`, mais retry com backoff para locks do Git causados por terceiros; commits dos Builders dentro da worktree ficam fora.
- **Supervisão:** `process-wrap` 10.x (process group / Job Object) com escalada própria: SIGTERM ou `CTRL_BREAK_EVENT` → `stop_grace_period` (default 10s, config global) → kill da árvore; o timeout do Agent Profile usa a mesma escalada. `KILL_ON_JOB_CLOSE` no Windows e `PR_SET_PDEATHSIG` no Linux.
- **Órfãos nunca são adotados:** o próximo comando que dirige verifica a identidade gravada, mata o grupo sobrevivente e só então cria o novo Attempt.
- **Abertura do DB:** migrations só sob `.surt/runtime/locks/db.lock`; WAL + `foreign_keys` + `busy_timeout`; `BEGIN IMMEDIATE` com retry; recusa DB com schema mais novo que o binário.
- **Daemon futuro:** cinco princípios registrados (Driver como função de biblioteca, nenhum estado só em memória, locks de arquivo, controle só pelo SQLite, sem adoção de agentes).
