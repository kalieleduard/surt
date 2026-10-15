# Contrato de agente

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 01, 02

## Question

Qual é o contrato completo de um agente — configuração, instruções, entradas, saídas e validação?

Decidir: schema de `agent.toml` (id, runtime, model, effort, timeout, perfis alternativos, permissões, ferramentas, parâmetros específicos do provider); como e quando um perfil é escolhido (ex.: Sonnet High por padrão, Opus Medium após N reprovações?); merge de defaults embutidos com overrides locais e snapshot da config efetiva; papel de `instructions.md` × prompt montado pelo Context Assembler; como o agente declara e entrega outputs (arquivos em caminhos combinados? JSON final validado por schema?) — em especial o manifesto de Tasks (ids, dependências) e a decisão estruturada do Reviewer (`APPROVED`/`CHANGES_REQUESTED` + findings); validação de modelo/effort/flags contra o Claude Code instalado; postura de permissões padrão para execução desacompanhada em worktree; a trait `AgentRuntime` e o que o Fake Runtime precisa simular.
