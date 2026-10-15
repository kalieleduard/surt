# Map: Surt — Arquitetura e plano de implementação

Labels: wayfinder:map

## Destination

O plano de engenharia completo do Surt — entregáveis A–L do brief (arquitetura, workspace Rust, domínio, schema SQLite, schema TOML de workflow, contratos de agente, máquinas de estado, artefatos/contexto, recuperação, CLI, roadmap, ADRs) mais a proposta objetiva do **Milestone 1** — escrito no repo em `docs/architecture/`, `docs/adr/` e `CONTEXT.md`, com cada decisão rastreável a um ticket resolvido. O mapa termina quando não resta nada a decidir para alguém começar a implementar o M1.

## Notes

- **Domínio:** Rust CLI local-first que orquestra agentes (Claude Code CLI) via workflows TOML declarativos; SQLite como fonte de verdade. O brief original é a referência de requisitos — está na primeira mensagem da sessão de charting e resumido nos tickets.
- **Skills por sessão:** `/grilling` + `/domain-modeling` em todo ticket grilling; `/codebase-design` nos tickets de estrutura de crates e traits (15); `/research` para tickets research.
- **Escrita incremental do plano (override de "plan, don't do"):** ao resolver um ticket, a sessão também escreve/atualiza a seção correspondente em `docs/architecture/` (ex.: `D-database.md`), registra ADRs em `docs/adr/` quando a decisão for difícil de reverter, surpreendente e fruto de trade-off real, e atualiza `CONTEXT.md` assim que um termo é resolvido. Não há ticket de "montagem" no final.
- **Idioma:** documentos do plano, ADRs, `CONTEXT.md`, identificadores, SQL e TOML em **inglês**; tickets e conversa em **PT-BR**.
- **Fidelidade:** SQL deve rodar em `sqlite3`; TOML deve fazer parse; contratos Rust em nível de assinatura + invariantes (sem corpo).
- **Plataformas:** macOS, Linux **e Windows** são alvo do MVP — toda decisão sobre processos, locks, paths, symlinks e worktrees deve valer nos três.
- **Restrições permanentes do brief:** núcleo não conhece Spec/Task/Builder/Reviewer/Shipper; sem tabelas específicas de agentes; sessão Claude nova por execução; nenhum Builder antes da aprovação humana; nenhuma Task concluída sem review aprovado; sem merge automático na branch principal; sem dependência de GitHub.
- **Crate atual** (`ferris-says` hello world) é descartável — será substituído pelo M1; o mapa não o altera.
- **Research:** achados ficam em `docs/research/<slug>.md` na branch descartável `research/<slug>`; o ticket aponta para ela.

## Decisions so far

<!-- the index — one line per closed ticket: enough to judge relevance, then zoom the link for the detail the ticket holds -->

## Not yet specified

- **Integrações externas** (GitHub Issues/PRs, Linear, Jira): por ora só um espaço reservado (`external_ref` em metadados) decidido no schema SQLite; o contrato de provider em si fica na névoa até o modelo de Task e artefatos estar firme.
- **Conteúdo do `config.toml` global** além do que cair em processo/concorrência, integração Git e CLI — consolidar quando esses tickets fecharem.
- **Rate limits e custo de tokens** como entrada do limite de concorrência do Scheduler.
- **Formato e retenção de `stream.jsonl` / `stderr.log`** — o que é persistido, por quanto tempo, e como interage com o cleanup e redação de segredos.

## Out of scope

- **Interface gráfica** — excluída pelo brief.
- **Daemon/supervisor local e `--detach`** — avaliado depois; o modelo de processo só precisa não impedi-lo.
- **Implementação de providers externos** (GitHub/Linear/Jira) — apenas espaço reservado no modelo.
- **Segundo runtime de IA real** (Codex, Gemini CLI etc.) — a trait é desenhada contra Claude Code + Fake Runtime.
- **Implementação do Milestone 1** — é o handoff ao fim do mapa, não parte dele.
- **Infra cloud, microsserviços, message brokers.**
