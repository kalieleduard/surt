# Capturar stream real do Claude Code como fixture

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: task
Status: open

## Question

Quais são os formatos reais dos eventos `stream-json` de um run **bem-sucedido** do Claude Code (eventos `system/init`, `assistant`, `user`, `result` com `structured_output`), e de falhas típicas (modelo inválido, `--max-turns` estourado, SIGTERM)?

A pesquisa de [Capacidades headless do Claude Code CLI](02-claude-code-cli-headless.md) não conseguiu observar um run real (OAuth expirado). Checklist (HITL — exige login válido do usuário ou `ANTHROPIC_API_KEY`):
1. `claude auth status` / re-login.
2. Rodar em diretório temporário: `claude -p "..." --output-format stream-json --verbose --permission-mode default --session-id <uuid> --no-session-persistence --json-schema '<schema simples>'` e salvar o stdout.
3. Repetir com modelo inválido, com `--max-turns 1` numa tarefa que precise de mais turnos, e enviando SIGTERM no meio.
4. Anotar exit codes e salvar os streams em `docs/research/fixtures/claude-code/` na branch `research/claude-code-cli-headless`.

Resultado alimenta o parser do runtime e o Fake Agent Runtime.
