# Capacidades headless do Claude Code CLI

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: research
Status: claimed (research subagent)

## Question

O que o Claude Code CLI instalado oferece, de fato, para execução não interativa controlada por um orquestrador?

Levantar com fontes primárias (docs oficiais, `claude --help`, changelog): flags de modelo e aliases (`--model`), como se define **effort** (flag, setting, env?), modos de permissão e allow/deny de ferramentas, `--output-format stream-json` (schema dos eventos, evento final com custo/uso/`session_id`), `--session-id`/geração de sessão nova garantida, `--append-system-prompt`/`--system-prompt`, saída estruturada validada por JSON Schema (se existe), exit codes e sinais, comportamento ao receber SIGTERM/SIGINT (e equivalente no Windows), working directory/`--add-dir`, como detectar versão e validar modelos/aliases antes de executar, suporte a Windows, variáveis de ambiente relevantes, limites de settings/hook que vazam do usuário (CLAUDE.md global, hooks) e como isolá-los.

Findings: `docs/research/claude-code-cli-headless.md` na branch `research/claude-code-cli-headless`.
