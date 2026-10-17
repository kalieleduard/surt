# Capturar stream real do Claude Code como fixture

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: task
Status: resolved
Assignee: Kaliel

## Question

Como são, de verdade, os eventos `stream-json` de um run bem-sucedido do Claude Code e das falhas que o runtime precisa classificar?

A pesquisa de [Capacidades headless do Claude Code CLI](02-claude-code-cli-headless.md) não observou um run real (OAuth expirado). Checklist (HITL — exige login válido):
1. `claude auth status` / re-login.
2. Em diretório temporário, rodar com a combinação de flags de [F. Agent Contract](../../../docs/architecture/F-agent-contract.md#claude-code-launcher-and-compatibility) (`dontAsk`, `--session-id` novo, `--json-schema` do envelope `outcome`/`outputs`) e salvar o stdout.
3. Repetir com: modelo inválido, `--max-turns 1` numa tarefa que precise de mais, SIGTERM no meio.
4. Anotar exit codes e salvar os streams em `docs/research/fixtures/claude-code/` na branch `research/claude-code-cli-headless`.

Conferir no stream de sucesso: `system/init` (modelo, permission mode, tools), `result` com `structured_output`, e se há eventos depois do `result`.

Resultado alimenta o parser do Claude Code Runtime e os roteiros do Fake Runtime. Rate limit e queda de rede ficam de fora: categoria desconhecida já cai em `retryable` ([I](../../../docs/architecture/I-recovery.md#transient-failures-network-and-rate-limits)).

## Answer

Feito em 2026-10-17 com Claude Code 2.1.301 (macOS, login por assinatura, `claude-haiku-5-5`, `--effort low`) e as flags de [F](../../../docs/architecture/F-agent-contract.md#claude-code-launcher-and-compatibility). Fixtures sanitizados, `capture.sh` e tabela de achados em `docs/research/fixtures/claude-code/` na branch `research/claude-code-cli-headless` (commit `378e0b3`, só local).

Sete cenários: `success`, `bad-model`, `auth-expired`, `max-turns`, `sigterm`, `schema-miss` (modelo recusa o structured output) e `permission-denied` (regra `Write(...)`).

| Cenário | Exit | `result` | Arquivos deixados |
|---|---|---|---|
| sucesso | 0 | `success`, `is_error=false`, `structured_output` válido | `note.md` |
| modelo inválido / auth expirada | 1 | `subtype=success` **com** `is_error=true`, `terminal_reason=api_error`; `assistant.error` = `model_not_found` / `authentication_failed` | nenhum |
| `--max-turns 1` | 1 | `error_max_turns` | todos (5 Writes paralelos num turno) |
| SIGTERM | 143 | **sem** `result`; stream termina num `tool_result` | parcial |
| recusa / escrita negada | 0 | `success` com envelope válido e prosa no lugar do path | nenhum |

O que mudou no plano (já escrito em [F](../../../docs/architecture/F-agent-contract.md#runtime-success-and-failure) e [I](../../../docs/architecture/I-recovery.md#transient-failures-network-and-rate-limits)):

- O lançador está validado ponta a ponta (assinatura, macOS). `--bare` e Windows seguem sem run real; ficam com a CI e o smoke manual de [16](16-estrategia-de-testes.md).
- Ambiente do filho por **allowlist**, não por remoção: `CLAUDE_CODE_SIMPLE`/`ANTHROPIC_BASE_URL` herdados de uma sessão Claude Code esconderam um login válido.
- `Write(path)` é ignorado em silêncio; só `Edit(//abs/**)` libera escrita (F já dizia, agora com evidência).
- `--json-schema` força um envelope mesmo quando o trabalho falhou: envelope válido não prova entrega; a checagem de arquivos (passo 5 de F) é o portão real.
- Checagem de `init`: o modelo pedido é ecoado mesmo inexistente; `plugins` continua populado em `--safe-mode` — checar `tools`/`mcp_servers`; `system/ui_invalidate` pode vir antes de `init`.
- `rate_limit_event` chega em todo run com `status: "allowed"`; só outro status é limite. Rate limit real e queda de rede não foram capturados (seguem `retryable` por padrão, como em I).

Nenhuma decisão nova de produto, ADR ou termo de `CONTEXT.md`.
