# Schema TOML de workflow

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Assignee: Kaliel
Blocked by: 01

## Question

Qual é o menor schema TOML que expressa o workflow de feature e o task-cycle sem virar uma linguagem?

Decidir: tipos de step (agent, human_approval, foreach/subworkflow, native) e seus campos; como se declaram **execution dependencies** (`needs`) × **data dependencies** (`inputs` referenciando artefatos de outros steps); como o foreach expande dinamicamente sobre Tasks que só existem depois do Task Agent rodar, respeitando dependências entre Tasks declaradas por elas mesmas; como o subworkflow recebe parâmetros e devolve outputs; condições/guards (ex.: decisão do reviewer) sem expressões arbitrárias; validação estática (ciclos, inputs inexistentes, tipos de step); versionamento do workflow e snapshot no run; workflows embutidos × overrides em `.surt/workflows/`; políticas de cleanup por workflow.

Saída: seção E com `feature.toml` e `task-cycle.toml` completos que fazem parse.

## Context

De [Glossário do domínio](01-glossario-do-dominio.md): o `foreach` não gera Step Runs achatados — cada item inicia um **Child Workflow Run** de uma Workflow Definition (ex.: `task-cycle`), e o Step Run do foreach só termina quando todos os filhos são terminais (ADR 0001). Iteration cria Step Runs novos; Step Run terminado nunca é reaberto — o schema TOML precisa expressar o loop sem ciclos no DAG sob essa regra.

## Answer

Resolvido em sessão de grilling. Documento: `docs/architecture/E-workflow-schema.md` (com `feature.toml` e `task-cycle.toml` completos, que fazem parse); trade-off em `docs/adr/0003-inputs-resolved-by-artifact-name.md`; termos **Outcome**, **Loop**, **Item List**, **Per-Item Step** e **Operation** (renomeado de Native Operation) em `CONTEXT.md`.

Decisões:
- **Não é linguagem:** cinco tipos de step (`agent`, `approval`, `operation`, `loop`, `per_item`), `kind` obrigatório; sem expressões, templating, JSONPath ou arestas de volta. Sintaxe `[steps.<id>]` (id único pelo próprio TOML); campo desconhecido = erro.
- **Outcome:** todo step termina com um valor de um conjunto fechado declarado (agente no `agent.toml`, `approved`/`rejected` no gate, operação no registro do Surt; `done` implícito). `when`/`until` só comparam Outcome por igualdade. `when` falso → SKIPPED (estado, não Outcome). Falha técnica não é Outcome.
- **Loop:** `kind = "loop"` com `body` e `until = { step, outcome }`; cada Iteration cria Step Runs novos para o corpo inteiro. **Sem `max_iterations` no TOML** (limite, se existir, é do ticket do loop). Rejeição num gate = nova volta do loop do gate (`spec-loop`, `tasks-loop`).
- **Inputs pelo nome, Revision mais recente** (ADR 0003), não "a Revision que o gate aprovou". A proteção contra ler algo não aprovado passa para a revalidação do ticket de artefatos: comando que dirige recusa (`✖ spec changed after approval (rev 2 not approved)`).
- **Contrato no `agent.toml`:** `outcomes`, `[inputs]` (com `optional = true`) e `[outputs]`; o step não repete a lista, só renomeia quando os nomes divergem. Entre Iterations o próximo agente lê a Revision mais nova pelo nome (`feedback`, `spec-rejection-reason`).
- **Motivo de rejeição** (`surt reject <feature> -m "…"`) vira Revision de origem humana do `rejection_artifact` do gate: `spec-rejection-reason`, `manifest-rejection-reason` (nomes por gate, senão o motivo de um gate vaza para o agente do outro).
- **Ordem explícita com `needs`**, nunca inferida; steps do corpo só dependem do mesmo corpo, o Loop carrega as dependências externas.
- **Per-Item Step** (`kind = "per_item"`, nome escolhido no lugar de `for_each`): `items` aponta para uma **Item List** de formato fixo (`key` + `needs`, resto é payload), validada quando o agente entrega (ciclo/chave duplicada → novo Attempt). `pass = [...]` declara o que o filho enxerga do pai (só leitura); o pai não lê artefatos dos filhos (o Shipper usa a branch de integração + `manifest`). Não existe step de subworkflow avulso.
- **Entradas do engine:** `request` (texto de `surt feature`, Revision humana) no root; `item` (referência a um item da Item List aprovada, não Revision nova) no filho.
- **Congelamento:** o root run congela o fecho (`feature` + `task-cycle`) na criação; identidade = SHA-256 da forma canônica (`feature@3f9a2c`); `schema = 1` é a versão do formato.
- **Cleanup:** só `[cleanup] on_success = "clean" | "keep"` (default `clean`); o que "clean" remove é do ticket de cleanup.
- **Validação estática** antes de todo run nascer (lista de 8 checagens na seção E).
- **Agent ids:** `spec`, `tasks`, `builder`, `reviewer`, `shipper`.
- **Fora do escopo:** override/customização de workflows embutidos e workflows próprios do projeto (só os embutidos no binário existem; o schema não impede).

## Comments

Extensão posterior aprovada em [Contrato de agente](07-contrato-de-agente.md): Item Lists admitem `files` por item, com formatos declarados em `agent.toml` e bytes incluídos na mesma Revision aprovada. A seção E foi atualizada; o mecanismo permanece genérico e não cria um novo kind de Step. Decisão detalhada e justificativa ficam no ticket do contrato e em [ADR 0004](../../../docs/adr/0004-item-list-files-share-approved-revision.md).
