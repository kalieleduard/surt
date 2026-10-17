# Roadmap + proposta do Milestone 1

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Blocked by: 13, 15, 16
Assignee: Kaliel

## Question

Qual é o menor Milestone 1 que já entrega uma Feature de ponta a ponta, e o que vem depois?

Decidir:
- **M1:** objetivo, módulos Rust, arquivos, testes e critério de pronto. Critério natural: o E2E do brief passa com o Fake Runtime e uma Feature pequena passa com o Claude Code de verdade.
- **Corte de comandos:** dos 14 de [J](../../../docs/architecture/J-cli.md), quais entram no M1. Candidatos a sair: `logs`, `inspect <execution-id>`, `--json`, rodapé ao vivo, `--new-run`, `stop` — sem quebrar exit codes e mensagens.
- **Corte de mecanismos já decididos:** o que foi desenhado nos tickets resolvidos mas pode ficar para depois sem mudar o schema (ex.: pausa por Transient Failure, `merge_into_base`, Salvage Refs, `restore`). Para cada um, confirmar que adiar não exige migration destrutiva.
- **Milestones seguintes:** ordem e objetivo em uma linha cada.

Candidatos de corte vindos de [I. Cleanup](../../../docs/architecture/I-cleanup.md): `on_success = "keep"`, salvage antes de remover worktree no `cancel`, `surt delete` (e a checagem de `--new-run` concorrente). `cleaned_at` e `.surt/logs/` já estão no schema/layout; adiar não exige migration.

De [B. Rust Workspace](../../../docs/architecture/B-workspace.md): os módulos do M1 saem da tabela de módulos de B (cada um pode entrar parcial); a ordem natural de construção é `platform`/`store`/`workflow` → `decide` + `drive` com `FakeRuntime` → `operations`/`git` → `reconcile`/`cleanup` → `ClaudeCodeRuntime`. A lista de dependências de B é o teto do M1, não o piso.

Saída: seções K e L (ADRs pendentes consolidados). Fecha o mapa.

## Answer

Resolvido em sessão de grilling (duas rodadas, todas as recomendações aceitas). Saída: [K. Roadmap and Milestone 1](../../../docs/architecture/K-roadmap.md) e [L. Architecture Decision Records](../../../docs/architecture/L-decisions.md). "Milestone 1" em A, G e J, onde queria dizer a v1 inteira, virou "v1"; J aponta para K para o subconjunto do M1.

Decisões:

- **Objetivo e pronto (Q1):** "uma Feature pequena vai de `surt feature` a uma Integration Branch entregue, com Claude Code de verdade, e sobrevive a `kill -9`". Pronto = suíte da [Estratégia de testes](16-estrategia-de-testes.md) verde na CI nos três SOs + smoke manual com Claude Code real **só em macOS** (Feature de 2 Tasks até `succeeded`, checklist em `docs/smoke-claude-code.md`).
- **Comandos (Q2):** 10 comandos — `init`, `feature` (criar com `--name`/`--base`/`--new`, retomar), `plan`, `start`, `approve`, `reject`, `cancel`, `status` (sem `--json`), `show` enxuto (`--rev`, `--task`). Fora: `stop` (Ctrl-C), `inspect`/`logs` (`status` mostra o caminho do log do step bloqueado), `restore` (apagar o arquivo o recria; recusa diz `fix: rm <file>`), `delete`, `--new-run`, `--json`, `--merge`/`--no-merge`. Exit codes e formato de mensagem intactos.
- **Mecanismos visíveis adiados (Q3):** Transient Failure bloqueia na hora sem gastar Attempt (`blocked_reason = 'not_retryable'`, exit 4, hora de reset na mensagem); `merge_into_base` só `"never"` (sem step `merge` no `feature.toml`); `on_success` só `"clean"`. Ficam: recuperação completa, Salvage Refs, Checks, cleanup com retry.
- **Mecanismos internos adiados (Q4):** overrides em `.surt/agents/` (driving commands recusam se existir), config global (`stop_grace_period` fixo 10 s), vagas entre processos (semáforo por Driver), rodapé/linha de 5 min/ASCII.
- **Forma de K e L (Q5):** K = roadmap + M1; L = índice das ADRs 0001–0012, sem ADR nova — cortes do M1 são subconjuntos reversíveis sem migration.
- **Fatias (Q6):** S1 Esqueleto → S2 Workflow → S3 Motor + Fake → S4 Git (E2E verde) → S5 Recuperação e fim; S6 Claude Code em paralelo a partir de S3. Um PR por fatia com CI verde.
- **Arquivos (Q7):** árvore de K; divisão de módulos de [B](../../../docs/architecture/B-workspace.md) travada, nomes de arquivo livres.
- **Depois do M1 (Q8):** M2 Operar e depurar → M3 Rodar sem babá → M4 Ajustar ao projeto; **v1 = fim do M4**.

Todos os cortes conferidos contra `0001_initial.sql`: nenhum exige migration.
