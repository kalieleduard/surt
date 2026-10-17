# Map: Surt — Arquitetura e plano de implementação

Labels: wayfinder:map

## Destination

O plano de engenharia completo do Surt — entregáveis A–L do brief (arquitetura, workspace Rust, domínio, schema SQLite, schema TOML de workflow, contratos de agente, máquinas de estado, artefatos/contexto, recuperação, CLI, roadmap, ADRs) mais a proposta objetiva do **Milestone 1** — escrito no repo em `docs/architecture/`, `docs/adr/` e `CONTEXT.md`, com cada decisão rastreável a um ticket resolvido. O mapa termina quando não resta nada a decidir para alguém começar a implementar o M1.

## Notes

- **Domínio:** Rust CLI local-first que orquestra agentes (Claude Code CLI) via workflows TOML declarativos; SQLite como fonte de verdade. O brief original é a referência de requisitos — está na primeira mensagem da sessão de charting e resumido nos tickets.
- **Skills por sessão:** `/grilling` + `/domain-modeling` em todo ticket grilling; `/codebase-design` nos tickets de estrutura de crates e traits (15); `/research` para tickets research.
- **Escrita incremental do plano (override de "plan, don't do"):** ao resolver um ticket, a sessão também escreve/atualiza a seção correspondente em `docs/architecture/` (ex.: `D-database.md`), registra ADRs em `docs/adr/` quando a decisão for difícil de reverter, surpreendente e fruto de trade-off real, e atualiza `CONTEXT.md` assim que um termo é resolvido. Não há ticket de "montagem" no final.
- **Estilo de grilling:** toda pergunta parte de um cenário real (ex.: "você roda `surt feature …`, cancela no meio…"), mostra o que o usuário veria em cada opção (esboço de `surt status`, linha do tempo) e a consequência prática; nada de pergunta só com definições abstratas.
- **Idioma:** documentos do plano, ADRs, `CONTEXT.md`, identificadores, SQL e TOML em **inglês**; tickets e conversa em **PT-BR**.
- **Fidelidade:** SQL deve rodar em `sqlite3`; TOML deve fazer parse; contratos Rust em nível de assinatura + invariantes (sem corpo).
- **Plataformas:** macOS, Linux **e Windows** são alvo do MVP — toda decisão sobre processos, locks, paths, symlinks e worktrees deve valer nos três.
- **Restrições permanentes do brief:** núcleo não conhece Spec/Task/Builder/Reviewer/Shipper; sem tabelas específicas de agentes; sessão Claude nova por execução; nenhum Builder antes da aprovação humana; nenhuma Task concluída sem review aprovado; sem merge automático na branch principal salvo opt-in `[git] merge_into_base = "ff-only"` (só fast-forward — [ADR 0006](../../docs/adr/0006-merge-into-base-is-opt-in-fast-forward.md)); sem dependência de GitHub.
- **Crate atual** (`ferris-says` hello world) é descartável — será substituído pelo M1; o mapa não o altera.
- **Research:** achados ficam em `docs/research/<slug>.md` na branch descartável `research/<slug>`; o ticket aponta para ela.

## Decisions so far

<!-- the index — one line per closed ticket: enough to judge relevance, then zoom the link for the detail the ticket holds -->

- [Capacidades headless do Claude Code CLI](issues/02-claude-code-cli-headless.md) — toda necessidade tem flag, mas sem validação prévia de modelo/effort, sem timeout de relógio, `-p` não isolado por padrão (`--bare` exige API key) e sucesso exige combinar exit code + `is_error` + `structured_output`.
- [Primitivas multiplataforma (macOS, Linux, Windows)](issues/18-primitivas-multiplataforma.md) — kill de árvore exige process group/Job Object, lock em arquivo separado via `File::lock` (Rust ≥1.89), identidade de PID = start time + boot id, `remove_dir_all` seguro contra links mas não idempotente; nomes reservados do Windows afetam nomes de feature.
- [SQLx + SQLite na prática](issues/03-sqlx-sqlite-na-pratica.md) — sqlx 0.9 empacota SQLite 3.46–3.51 (STRICT/RETURNING/JSON ok); não liga WAL; `begin()` é DEFERRED (usar `BEGIN IMMEDIATE`); migração concorrente não é protegida e checksum quebra com CRLF; timestamps de chrono e de `CURRENT_TIMESTAMP` não comparam.
- [Mecânica de worktree/merge no Git e estados parciais](issues/04-git-worktree-merge-estados-parciais.md) — comandos git não são idempotentes (reconciliar lendo estado); `worktree add -b` não é atômico; integração sem worktree via `merge-tree`+`commit-tree`+`update-ref` é reexecutável (Git ≥2.38); merge/rebase em andamento só por marcadores; CLI `git` > libgit2/gix.
- [Glossário do domínio](issues/01-glossario-do-dominio.md) — núcleo genérico sobre Subject → Workflow Run (congela a Definition; ≤1 ativo por Subject) → Step Run → Execution; cada Task é um Child Workflow Run do `task-cycle`; Iteration cria Step Runs novos, Attempt cria Execution nova; Approval Gate + Approval sobre Revision imutável.
- [Modelo de processo e controle de concorrência](issues/05-modelo-de-processo-e-concorrencia.md) — `surt feature`/`plan`/`start` dirigem uma fase cada em foreground e `approve` só registra; um Driver por root run garantido por lock de arquivo (sem lease/heartbeat); Ctrl-C = Interruption retomável; controle externo só pelo SQLite; vagas de agente por projeto como lock files; órfãos nunca adotados.
- [Schema TOML de workflow](issues/06-schema-toml-de-workflow.md) — cinco `kind` sem expressões (`agent`, `approval`, `operation`, `loop`, `per_item`); condições só comparam Outcome; rejeição e review reprovado = nova Iteration de um Loop (sem `max_iterations`); inputs pelo nome na Revision mais recente (ADR 0003, proteção na revalidação); `needs` explícito; Item List fixa (`key` + `needs`); root congela o fecho por SHA-256.
- [Contrato de agente](issues/07-contrato-de-agente.md) — contratos/perfis congelados; arquivos + JSON final validado; `.md` por Task junto ao manifesto aprovado; `dontAsk`; sem timeout de execução.
- [Armazenamento de artefatos, congelamento e revalidação](issues/08-armazenamento-congelamento-revalidacao.md) — blobs SHA-256 no SQLite (publicação atômica, Approval = id + hash); Working Copy editável só para artefatos com gate, edições detectadas por hash viram Revision humana; Revision aprovada e já lida como Input fica fixada e edição posterior é recusada (ADR 0005); inputs por Execution em `runtime/executions/<id>/`; `.surt/` ignorado salvo `config.toml`/`agents/`; nome `YYYYMMDD-<slug>` com sufixo `-N`.
- [Modelo de integração Git](issues/09-modelo-de-integracao-git.md) — Operation nativa integra o SHA revisado como merge commit (`merge-tree`/`commit-tree`/`update-ref`, Git ≥2.38) em `surt/<subject>`, re-checando se a ponta andou; Tasks em `surt/task/<subject>/<key>` partindo da ponta; Base = `HEAD` em `surt feature`; Checks argv congelados, `failed` pula review; conflito volta ao Builder com review completo; Shipper só leitura (`title`/`summary`) após `snapshot`; merge na Base só opt-in ff-only (ADR 0006/0007).
- [Máquina de estados do loop Builder ↔ Reviewer](issues/10-loop-builder-reviewer.md) — `blocked` (não final) no lugar de falha: 3 Attempts e 5 Iterations (sem limite em Loop com gate), retomar o comando renova o orçamento; run só termina `succeeded`/`cancelled` (ADR 0008); dependentes de Task bloqueada esperam; um único perfil por agente, sem escalada; Builder sem mudança faz commit vazio; Reviewer lê `feedback?`/`conflict?` com veredito sempre do SHA inteiro.
- [Schema SQLite](issues/11-schema-sqlite.md) — 17 tabelas no singular (sem `step_dependencies`; `execution_input` no lugar de `execution_artifacts`); UUIDv7 TEXT, timestamps TEXT ISO de 24 chars gravados pelo Rust, STRICT, WAL + `synchronous=FULL`; sem `deleted_at` — purge em cascata (ADR 0009); snapshots content-addressed; `commit-tree` datado por `step_run.started_at`; linhas nascem `pending` de uma vez (run, Iteration, expansão); JSON só onde o engine não decide.

## Not yet specified

- **Integrações externas** (GitHub Issues/PRs, Linear, Jira): o [Schema SQLite](issues/11-schema-sqlite.md) decidiu **não** reservar coluna (`external_ref` entra por migration aditiva quando houver provider); o contrato de provider em si fica na névoa.
- **Conteúdo do `config.toml` global e do config de projeto** — já existem `stop_grace_period` (global), `max_concurrent_agents`, `max_attempts`, `max_iterations`, `[[checks]]` e `[git] merge_into_base` (projeto, congelados no root run como snapshot `config` — [D. Database Schema](../../docs/architecture/D-database.md#snapshots)); consolidar com o que cair na CLI quando esse ticket fechar.
- **Rate limits e custo de tokens** como entrada do limite de concorrência do Scheduler — o limite decidido é por projeto, então vários projetos na mesma máquina podem somar acima do rate limit da conta.

- **Validação em Windows real**: reação do `claude.exe` a Ctrl+Break, `wait` do process-wrap no Windows, `core.longpaths` do Git for Windows (e a injeção via `GIT_CONFIG_*` no agente) — vira ticket quando houver máquina/CI Windows disponível (provável no roadmap/estratégia de testes).

## Out of scope

- **Interface gráfica** — excluída pelo brief.
- **Daemon/supervisor local e `--detach`** — avaliado depois; o modelo de processo só precisa não impedi-lo.
- **Implementação de providers externos** (GitHub/Linear/Jira) — apenas espaço reservado no modelo.
- **Segundo runtime de IA real** (Codex, Gemini CLI etc.) — a trait é desenhada contra Claude Code + Fake Runtime.
- **Override/customização de workflows embutidos e workflows próprios do projeto** — só `feature` e `task-cycle` embutidos no binário; caso de borda, o schema não impede ([Schema TOML de workflow](issues/06-schema-toml-de-workflow.md)).
- **Implementação do Milestone 1** — é o handoff ao fim do mapa, não parte dele.
- **Cancelar ou abandonar uma Task só, com entrega parcial** — no M1 filhos só terminam junto com o root, e o Shipper entrega só Features completas ([Máquina de estados do loop Builder ↔ Reviewer](issues/10-loop-builder-reviewer.md)).
- **Infra cloud, microsserviços, message brokers.**
