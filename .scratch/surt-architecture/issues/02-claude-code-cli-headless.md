# Capacidades headless do Claude Code CLI

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: research
Status: resolved

## Question

O que o Claude Code CLI instalado oferece, de fato, para execução não interativa controlada por um orquestrador?

Levantar com fontes primárias (docs oficiais, `claude --help`, changelog): flags de modelo e aliases (`--model`), como se define **effort** (flag, setting, env?), modos de permissão e allow/deny de ferramentas, `--output-format stream-json` (schema dos eventos, evento final com custo/uso/`session_id`), `--session-id`/geração de sessão nova garantida, `--append-system-prompt`/`--system-prompt`, saída estruturada validada por JSON Schema (se existe), exit codes e sinais, comportamento ao receber SIGTERM/SIGINT (e equivalente no Windows), working directory/`--add-dir`, como detectar versão e validar modelos/aliases antes de executar, suporte a Windows, variáveis de ambiente relevantes, limites de settings/hook que vazam do usuário (CLAUDE.md global, hooks) e como isolá-los.

Findings: `docs/research/claude-code-cli-headless.md` na branch `research/claude-code-cli-headless`.

## Answer

Achados completos: `docs/research/claude-code-cli-headless.md` na branch `research/claude-code-cli-headless` (commit `172db83`; Claude Code v2.1.300, macOS). **Ressalva:** o OAuth local estava expirado, então nenhum run chegou ao modelo — formatos de eventos de sucesso e `structured_output` vêm só da documentação.

Fatos que restringem o design (não são decisões):
- Todo controle necessário tem flag: `--model`, `--effort`, `--permission-mode`/`--tools`/`--allowedTools`/`--disallowedTools`, `--output-format stream-json --verbose` (sem `--verbose` sai com 1), `--json-schema`, `--session-id` (UUID; recusa id já existente), `--no-session-persistence`, `--system-prompt[-file]`/`--append-system-prompt[-file]`, `--max-turns`, `--max-budget-usd`, isolamento via `--bare`/`--settings`/`--setting-sources`/`--strict-mcp-config`.
- Sucesso exige combinar exit code + `is_error` + `subtype` + presença de `structured_output`; o `result` pode não ser a última linha; o parser deve tolerar tipos/campos desconhecidos.
- Não há como validar modelo/effort antes do launch pela CLI; effort inválido é ignorado com warning; aliases variam por versão/provider — fixar IDs completos; modelo resolvido aparece em `system/init`.
- Modo de permissão padrão depende de estado oculto — sempre passar `--permission-mode` explícito; `bypassPermissions` é recusado como root.
- `-p` padrão **não é isolado** (hooks, MCP, plugins, CLAUDE.md do usuário carregam); `--bare` isola mas exige `ANTHROPIC_API_KEY`/`apiKeyHelper` (não lê login de assinatura); `CLAUDE_CONFIG_DIR` próprio também move credenciais.
- SIGTERM → exit 143, mata árvores Bash, sem resultado do turno; **não há timeout de relógio** (o Surt precisa impor o seu); run pode ficar aberto até ~10 min esperando trabalho em background. Semântica de parada no Windows não documentada.
- Releases quase diárias; `claude --version` funciona sem auth; `system/init` expõe versão e `capabilities`; auto-update pode trocar o binário entre runs (desligável por env).

Perguntas abertas que seguem para outros tickets: postura de auth × isolamento e timeouts (Contrato de agente), semântica de parada no Windows (Primitivas multiplataforma / Modelo de processo), fixture de stream real (novo ticket 19).
