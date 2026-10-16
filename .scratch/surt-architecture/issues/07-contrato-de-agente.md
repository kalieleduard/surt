# Contrato de agente

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 01, 02

## Question

Qual é o contrato completo de um agente — configuração, instruções, entradas, saídas e validação?

Decidir: schema de `agent.toml` (id, runtime, model, effort, timeout, perfis alternativos, permissões, ferramentas, parâmetros específicos do provider); como e quando um perfil é escolhido (ex.: Sonnet High por padrão, Opus Medium após N reprovações?); merge de defaults embutidos com overrides locais e snapshot da config efetiva; papel de `instructions.md` × prompt montado pelo Context Assembler; como o agente declara e entrega outputs (arquivos em caminhos combinados? JSON final validado por schema?) — em especial o manifesto de Tasks (ids, dependências) e a decisão estruturada do Reviewer (`APPROVED`/`CHANGES_REQUESTED` + findings); validação de modelo/effort/flags contra o Claude Code instalado; postura de permissões padrão para execução desacompanhada em worktree; a trait `AgentRuntime` e o que o Fake Runtime precisa simular.

## Context

Surgiu de [Capacidades headless do Claude Code CLI](02-claude-code-cli-headless.md) — também decidir aqui:
- **Auth × isolamento:** `--bare` (isolamento total, exige API key) × flags combinadas mantendo login de assinatura (`--setting-sources`, `--settings`, `--strict-mcp-config`, `CLAUDE_CODE_DISABLE_CLAUDE_MDS`) × `CLAUDE_CONFIG_DIR` por run. Qual é o padrão e o que vai para `agent.toml`?
- **Validação de modelo/effort** já que a CLI não valida antes do launch: allowlist própria versionada? checagem pós-`system/init`?
- **Timeout de relógio** imposto pelo Surt e relação com `--max-turns`/`--max-budget-usd`.
- **Versão mínima/pinagem do Claude Code** e desligar auto-update durante runs.
- **Regra de sucesso** da execução (exit code + `is_error` + `subtype` + `structured_output`).
