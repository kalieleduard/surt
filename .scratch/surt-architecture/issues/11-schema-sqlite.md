# Schema SQLite

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 01, 03, 05, 06, 08

## Question

Qual é o schema físico completo (7 tabelas) que serve a qualquer workflow e preserva integridade referencial sem JSON para relações críticas?

Decidir: colunas, tipos, PK UUIDv7, FKs e `ON DELETE`; o que `deleted_at` significa em cada tabela (inclusive `events`, que deveria ser append-only); constraints de status (CHECK) e unicidade (ex.: uma execution ativa por step; `(workflow_run_id, logical_key)`; `(step_run_id, iteration, attempt)`); `step_dependencies` e `execution_artifacts` (papel input/output); revisões e hashes de artefatos; onde moram approval, lease/heartbeat, `external_ref`; índices para o scheduler e para `surt status`; estratégia de migrations; o que fica em JSON (snapshots, payloads) e o que não pode ficar.

Saída: seção D com SQL que roda em `sqlite3`, ERD Mermaid e migration inicial.

## Context

De [Contrato de agente](07-contrato-de-agente.md): persistir snapshots de agentes/perfis/instruções/schemas e o perfil escolhido por Execution, sem credenciais. Uma Revision de Item List abrange índice e arquivos referenciados; filhos apontam para a mesma Revision e seus arquivos, sem nova origem nem tabelas específicas de Tasks. Formato físico e endereçamento dos bytes são do ticket de artefatos; este ticket deve representar essas relações. Ver [contrato canônico](../../../docs/architecture/F-agent-contract.md).

Surgiu de [SQLx + SQLite na prática](03-sqlx-sqlite-na-pratica.md) — decidir aqui: UUID como BLOB × TEXT; formato canônico de timestamp (TEXT ISO fixo × INTEGER ms) aplicado igual por Rust e por defaults SQL; tabelas STRICT; política de migrations (LF forçado via `.gitattributes`, só-adição, `-- no-transaction` para rebuilds, comportamento de binário antigo com DB novo); quem e quando habilita WAL na criação do DB; `query!` com `.sqlx` × `query_as` em runtime.

De [Glossário do domínio](01-glossario-do-dominio.md): a unicidade sugerida `(step_run_id, iteration, attempt)` não vale mais — Iteration pertence ao **Step Run** (novos Step Runs por rodada), Attempt à Execution. O schema precisa de: relação pai→filho entre Workflow Runs (com o Step Run que iniciou o filho); representação do **Subject** com ≤1 run raiz ativo; Approval como registro próprio apontando para uma Artifact Revision; origem única por Revision (uma Execution ou uma edição humana); Inputs/Outputs de Execution para Revisions.

De [Modelo de processo e controle de concorrência](05-modelo-de-processo-e-concorrencia.md): o root run precisa de identidade do Driver (pid, start time nativo, boot id), só informativa, e de um cancel request; cada Execution de agente precisa da identidade do processo e da vaga ocupada (`agent-<i>`), também só informativa. Nenhuma lease/heartbeat: a propriedade é decidida por lock de arquivo. Migrations rodam sob `db.lock`; DB com schema mais novo que o binário é recusado.

De [Schema TOML de workflow](06-schema-toml-de-workflow.md): artefatos são identificados por **nome dentro de um Workflow Run** (unicidade `(workflow_run_id, name)`), e inputs resolvem para a Revision mais recente no início da Execution (gravar a Revision fixada por Execution). Duas entradas do engine: `request` (Revision de origem humana no root) e `item` (referência a um item, por `key`, de uma Revision de Item List do pai — não é Revision nova). O motivo de `surt reject` é Revision humana do `rejection_artifact`. O root run guarda o snapshot congelado do fecho de definições (`feature` + `task-cycle`) com SHA-256 da forma canônica; Child Runs referenciam o snapshot do root. Step Runs pertencem a uma Iteration de um Loop (número da Iteration no Step Run); estado SKIPPED.

De [Armazenamento de artefatos, congelamento e revalidação](08-armazenamento-congelamento-revalidacao.md): bytes de Revisions vivem no SQLite como blobs content-addressed (`blobs(sha256, bytes)` + arquivos por Revision; esboço testado em [H. Artifact Storage](../../../docs/architecture/H-artifact-storage.md)); publicar Revision = uma transação. Faltam aqui: tabelas de Revision (identidade = SHA-256 dos pares `(rel_path, sha256)`), Approval (revision_id + hash), **baseline** por arquivo da Working Copy, marca de **Pinned Revision** (primeira leitura como Input após aprovação) e diretórios de Execution.

De [Modelo de integração Git](09-modelo-de-integracao-git.md) ([A. Git Integration](../../../docs/architecture/A-git-integration.md)): persistir o `project_id` (`<dir-slug>-<6hex>`, gerado ao criar o `surt.db`); por Child Run de Task: caminho absoluto da worktree, nome da Task Branch e SHA de `start`; por root run: nome da Integration Branch (com `.N` a partir do segundo run do Subject, então o ordinal do run dentro do Subject precisa existir), ponta gravada (para o CAS e para detectar movimento externo) e a config de projeto congelada (`[[checks]]`, `[git] merge_into_base`). `base` é um Initial Artifact (Revision human-origin criada junto com `request` na criação do root run). Operations são Executions: datas fixas do `commit-tree` = início gravado da Execution.

De [Máquina de estados do loop Builder ↔ Reviewer](10-loop-builder-reviewer.md) ([G. State Machines](../../../docs/architecture/G-state-machines.md)): estados com CHECK — Execution `queued|running|succeeded|failed|interrupted|cancelled`; Step Run `pending|running|done|skipped|blocked|cancelled` (`blocked` não é final); Workflow Run grava só `active|succeeded|cancelled` + pedido de cancel (quando, por quem), e `RUNNING/AWAITING APPROVAL/BLOCKED/INTERRUPTED` são derivados na leitura (índices para isso). Contadores de Attempt e de Iteration guardam o histórico inteiro, mas o orçamento é medido desde o último desbloqueio (gravar quando o Step Run voltou a `running`). `max_attempts`/`max_iterations` fazem parte da config de projeto congelada no root run. **Agora há um único Agent Profile por Agent** (sem `default_profile`, sem perfis nomeados): o snapshot do agente contém um perfil e a Execution só referencia esse snapshot.
