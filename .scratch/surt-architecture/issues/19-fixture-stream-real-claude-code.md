# Capturar stream real do Claude Code como fixture

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: task
Status: open

## Question

Como são, de verdade, os eventos `stream-json` de um run bem-sucedido do Claude Code e das falhas que o runtime precisa classificar?

A pesquisa de [Capacidades headless do Claude Code CLI](02-claude-code-cli-headless.md) não observou um run real (OAuth expirado). Checklist (HITL — exige login válido):
1. `claude auth status` / re-login.
2. Em diretório temporário, rodar com a combinação de flags de [F. Agent Contract](../../../docs/architecture/F-agent-contract.md#claude-code-launcher-and-compatibility) (`dontAsk`, `--session-id` novo, `--json-schema` do envelope `outcome`/`outputs`) e salvar o stdout.
3. Repetir com: modelo inválido, `--max-turns 1` numa tarefa que precise de mais, SIGTERM no meio.
4. Anotar exit codes e salvar os streams em `docs/research/fixtures/claude-code/` na branch `research/claude-code-cli-headless`.

Conferir no stream de sucesso: `system/init` (modelo, permission mode, tools), `result` com `structured_output`, e se há eventos depois do `result`.

Resultado alimenta o parser do Claude Code Runtime e os roteiros do Fake Runtime. Rate limit e queda de rede ficam de fora: categoria desconhecida já cai em `retryable` ([I](../../../docs/architecture/I-recovery.md#transient-failures-network-and-rate-limits)).
