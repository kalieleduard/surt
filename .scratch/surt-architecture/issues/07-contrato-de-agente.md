# Contrato de agente

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Assignee: Kaliel
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

De [Schema TOML de workflow](06-schema-toml-de-workflow.md): o `agent.toml` é dono do contrato. Declara `outcomes = [...]` (conjunto fechado; um agente com um só resultado tem o Outcome implícito `done`), `[inputs]` (nome → `format`, `optional = true` quando pode faltar, ex.: `feedback` na volta 1) e `[outputs]` (nome → `format`). O workflow não repete a lista: o Surt entrega a Revision mais recente de cada artefato com o mesmo nome, com renomeação opcional no step quando os nomes não batem. Validação estática do workflow carrega os agentes.

## Comments

### Rodada 1 — escolhas confirmadas

O humano respondeu `Q1 OK`, `Q2 OK`, `Q3 - OK`:

- **Autenticação:** login existente da assinatura como padrão, desativando explicitamente configuração herdada onde a CLI permite; isolamento parcial reconhecido. API key com `--bare` como opção explícita.
- **Entrega:** artefatos em arquivos mais JSON final obrigatório declarando Outcome e arquivos entregues; Surt valida ambos. Arquivo sem JSON válido é falha técnica; `changes_requested` válido é sucesso com esse Outcome.
- **Congelamento:** na criação do root run, congelar contratos, instruções e perfis efetivos de todos os agentes utilizados, após overrides locais. Cada Execution registra o perfil escolhido do conjunto congelado; alterações locais valem para novos runs.

Seção incremental: [F. Agent Contract](../../../docs/architecture/F-agent-contract.md). O contrato completo ainda não foi resolvido; o ticket permanece assumido para as próximas rodadas.

### Rodada 2 — overrides, perfis e permissões confirmados; timeout excluído

O humano respondeu `Q4 - OK`, `Q5 - OK`, `Q6 - OK`, `Q7 - Não colocar timeout agora`:

- **Overrides:** defaults embutidos mais override local por campo; listas substituídas integralmente; campos desconhecidos são erros. `instructions.md` local substitui o texto embutido inteiro.
- **Perfis:** nomes e default explícitos no agente; novo Attempt por falha técnica mantém o perfil; nova Iteration pode selecionar outro perfil congelado por política do loop. Limiares pertencem a [Máquina de estados do loop Builder ↔ Reviewer](10-loop-builder-reviewer.md). Runtime não troca silenciosamente de modelo para contornar erro.
- **Permissões:** `dontAsk` com ferramentas e autorizações explícitas por perfil. Chamada não autorizada é negada; se impedir a entrega, falha técnica com diagnóstico. `bypassPermissions` é alternativa explícita. Sem promessa de sandbox do sistema operacional.
- **Sem timeout de execução agora:** sem campo, default ou temporizador de duração total no contrato atual. Isso revisa a previsão anterior do modelo de processo. `stop_grace_period` continua valendo após Interruption/Cancellation; não é timeout de execução. Limites de turnos e custo ainda não foram decididos.

Decisões incorporadas em [F. Agent Contract](../../../docs/architecture/F-agent-contract.md), com ajuste de consistência em [A. Process Model and Concurrency](../../../docs/architecture/A-process-model.md).

### Rodada 3 — contexto, envelope, modelo e limites confirmados

O humano respondeu `OK para todas` às perguntas Q8–Q11:

- **Instruções e contexto:** `instructions.md` contém o papel estável; Context Assembler monta Inputs com Revisions fixadas, item atual, diretório de trabalho e contrato de entrega de cada Execution. Sem templates ou expressões nas instruções.
- **Envelope único:** `{"outcome":"approved","outputs":{"feedback":"feedback.json"}}`; nomes e Outcomes vêm do contrato; caminhos relativos ao diretório de entrega da Execution; sem escapes nem links simbólicos. Surt valida formatos/schemas e só aceita a entrega completa; inconsistência é falha técnica.
- **Modelo/effort:** IDs completos, sem aliases móveis; validação local de valores conhecidos de effort; conferir modelo informado na inicialização. Incompatibilidade/indisponibilidade é falha técnica, sem escolher outro modelo. Sem catálogo próprio por conta; effort solicitado é registrado, sem promessa de verificar o efetivamente aplicado.
- **Limites:** turnos e custo opcionais por perfil, sem limites padrão; enviados à CLI apenas se configurados. Atingir limite é falha técnica; custo é aproximado e não garante cobrança máxima. Continua sem timeout de execução.

Seção incremental atualizada em [F. Agent Contract](../../../docs/architecture/F-agent-contract.md).

### Rodada 4 — entregáveis, perfis, runtime e compatibilidade confirmados

O humano confirmou Q12–Q15, acrescentando: `cada task terá que ter um .md para passar ao builder posteriormente`.

- **Entregáveis:** Spec entrega Markdown; Tasks entrega Item List com `key`, `needs`, `title`, `description`, `acceptance_criteria`; Builder entrega SHA em JSON; Reviewer entrega commit revisado, resumo e findings (`blocking`, `path`, `message`, `line` opcional); Shipper entrega branch, SHA final e resumo em JSON. Outcome permanece no envelope. `approved` com finding bloqueante é inconsistente. Além do manifesto, cada Task exige seu próprio `.md`; o empacotamento/congelamento desses arquivos ainda será decidido na rodada seguinte.
- **Perfis embutidos:** `normal` (default) Sonnet/high; `reinforced` Opus/medium nos cinco agentes. IDs completos confirmados em fonte oficial: `claude-sonnet-5-5` e `claude-opus-5-5`; disponibilidade da conta continua sendo verificada no runtime.
- **Runtime:** lança, acompanha e para uma Execution; Surt valida a entrega e avança o fluxo. Fake Runtime gera arquivos/eventos simulados pelo mesmo caminho de validação, sem alterar diretamente estados no SQLite.
- **Compatibilidade:** baseline mínima 2.1.301, recursos conferidos e versão efetiva por Execution; versões posteriores aceitas quando compatíveis, sem pinagem exata por run. Auto-update desativado nos processos lançados. Inspeção confirmou flags, sem execução autenticada completa; fixture real continua pendente em ticket próprio.

Seção [F. Agent Contract](../../../docs/architecture/F-agent-contract.md) consolidada com schema TOML, schemas JSON e invariantes. Ticket permanece assumido enquanto se fecha a entrega dos `.md` por Task.

### Rodada 5 — Markdown por Task confirmado

O humano respondeu `ISSO` à Q16: o Task Agent escreve cada `.md`, o item o referencia por `files.task`, e manifesto mais arquivos formam uma única Revision aprovada. Builder recebe o arquivo da própria Task, fixado à Revision aprovada. Arquivo ausente/inválido impede a entrega; edição cria nova Revision e exige reaprovação. O núcleo trata anexos genéricos, não Tasks.

## Answer

Resolvido por troca humana nas cinco rodadas acima. Contrato completo em [F. Agent Contract](../../../docs/architecture/F-agent-contract.md), schemas em [schemas/](../../../docs/architecture/schemas/), extensão genérica de Item Lists em [E. Workflow Definition Schema](../../../docs/architecture/E-workflow-schema.md) e trade-off em [ADR 0004 — Item List files share the approved Revision](../../../docs/adr/0004-item-list-files-share-approved-revision.md). Glossário e seções A/C alinhados.

Decisões finais:

- **Configuração e congelamento:** defaults embutidos + override local por campo, listas substituídas, campos desconhecidos rejeitados; `instructions.md` local substitui o texto completo. Root run congela agentes, contratos, perfis, instruções e schemas; cada Execution registra o perfil escolhido.
- **Perfis:** `normal` Sonnet/high e `reinforced` Opus/medium nos cinco agentes, com IDs completos. Attempt técnico mantém perfil; outra Iteration pode escolher outro por política do loop. Limiares permanecem no ticket do loop.
- **Auth e permissões:** login de assinatura como padrão, com isolamento parcial explícito; API key + `--bare` como alternativa. `dontAsk`, tools/regras explícitos por perfil; bypass somente explícito. Sem promessa de sandbox do SO.
- **Entrega:** arquivos + envelope JSON obrigatório `outcome`/`outputs`; validação independente do envelope, arquivos, formatos/schemas e consistência. `changes_requested` válido é sucesso; falha técnica não é Outcome.
- **Tasks:** um `.md` escrito pelo Task Agent para cada item, por `files.task`. JSON e arquivos compartilham a Revision/Approval; cada filho recebe só os seus arquivos. Sem tabelas/branches de lógica específicas de agentes no núcleo.
- **Limites:** nenhum timeout de execução agora. Turnos/custo opcionais, sem default; `stop_grace_period` continua sendo apenas a sequência de parada explícita.
- **Contexto e runtime:** instruções estáveis separadas do contexto montado com Inputs fixados. Runtime lança/acompanha/para uma Execution; Fake gera arquivos/eventos pelo mesmo caminho de validação, sem manipular estado SQLite.
- **Compatibilidade e aceitação:** CLI >=2.1.301 com controles compatíveis, versão por Execution, auto-update desativado nos filhos e sessão UUID nova por lançamento. Aceitar exige init compatível, exit 0, resultado final sem erro e entrega válida; tolerar eventos/campos novos e drenar stream até o fim.

Validação documental: exemplos TOML/JSON parseados, schemas JSON parseados e links locais conferidos. Não houve implementação do runtime nem execução de sessão autenticada; [Capturar stream real do Claude Code como fixture](19-fixture-stream-real-claude-code.md) continua aberto para validar a combinação real. Retenção/redação foi especificada como novo ticket [Política de logs, retenção e redação](13-cleanup-e-ciclo-de-vida.md) (depois absorvido pelo 13), que passa a bloquear o cleanup. Persistência/revalidação, prova Git, política do loop e representação das traits continuam nos tickets já existentes.
