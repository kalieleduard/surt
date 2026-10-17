# Schema SQLite

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Blocked by: 01, 03, 05, 06, 08
Assignee: Kaliel

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

## Answer

Resolvido por troca humana em duas rodadas, mais revisões pedidas sobre exemplos completos do schema. Seção completa em [D. Database Schema](../../../docs/architecture/D-database.md); migration inicial testada em [`migrations/0001_initial.sql`](../../../docs/architecture/migrations/0001_initial.sql); trade-off em [ADR 0009 — History is purged, not soft-deleted](../../../docs/adr/0009-history-is-purged-not-soft-deleted.md).

Decisões finais:

- **17 tabelas, nomes no singular**, não as 7 do brief: `project`, `blob`, `snapshot`, `snapshot_file`, `run_snapshot`, `subject`, `workflow_run`, `step_run`, `execution`, `execution_input`, `artifact`, `artifact_revision`, `revision_file`, `approval`, `working_copy_file`, `run_branch`, `event`. `step_dependencies` não existe (os `needs` vêm da definição congelada); `execution_artifacts` virou `execution_input` (saída = origem da Revision).
- **Representação:** UUIDv7 em TEXT hifenizado; timestamps TEXT UTC de 24 caracteres (`…T14:02:11.483Z`) sempre gravados pelo Rust, sem `CURRENT_TIMESTAMP`; `event.seq` INTEGER autoincremento; tudo `STRICT`.
- **Sem `deleted_at`:** purge = um `DELETE FROM subject` em cascata + GC de snapshots e blobs órfãos. Composição em `CASCADE`, referências cruzadas em `NO ACTION` (testado: `RESTRICT` quebraria a cascata). `event` append-only por trigger (ADR 0009).
- **Snapshots content-addressed** para workflow, agente e config de projeto, compartilhados entre runs; `run_snapshot` registra o fecho de cada root run.
- **`commit-tree` usa `step_run.started_at`**, não o início da Execution: a seção A dizia que um novo Attempt recalculava o mesmo `M`, o que era falso com datas por Execution. [A. Git Integration](../../../docs/architecture/A-git-integration.md#integration) corrigida.
- **Conexão:** WAL ligado na criação sob `db.lock`, `synchronous = FULL` (o registro de intenção não pode sumir depois do efeito), `BEGIN IMMEDIATE`. Queries `query_as` em runtime, sem `.sqlx`; migrations embutidas, LF forçado, nunca editadas.
- **Step Run:** `budget_base` (falhas ou Iterations contadas no último desbloqueio) e `blocked_reason` ∈ `attempts_exhausted | not_retryable | iterations_exhausted`; o detalhe vem do `error_message` da última Execution.
- **Approval:** uma decisão por Revision; `gate_step_run_id` NULL para aprovação de revalidação; reject exige a Revision do motivo. `actor` = `git user.email`, com fallback no usuário do sistema.
- **Pin:** só `pinned_at`, gravado na transação que fixa o primeiro Input ou expande a Item List (a expansão pina antes de existir Execution); motivo em evento.
- **`run_branch` tipada** (uma branch por run, `ref_name` único, `tip_sha` para CAS, `worktree_path`, `merged_as_sha`).
- **JSON** só em `execution.runtime_info` e `event.data`, que nenhuma decisão do engine lê. `event` é auditoria transacional, nunca fonte de verdade, com `kind` aberto.
- **`project`:** `project_id` (`<slug>-<6hex>`, legível porque nomeia `~/.surt/worktrees/<project-id>/`) é a PK; o trigger `project_single_row` recusa uma segunda linha. Nenhuma coluna `singleton`.
- **Nascimento das linhas:** Step Runs do topo nascem `pending` com o run; os do corpo de um Loop, no início de cada Iteration; os runs filhos, todos na expansão do Per-Item Step (junto com o pin); `artifact` só nasce com a primeira Revision.
- **Purge testado de ponta a ponta:** `snapshot_file → snapshot` é composição (`CASCADE`), para o GC de snapshots órfãos funcionar; `run_snapshot → snapshot` continua `NO ACTION`, então um snapshot em uso não pode ser apagado.
- Colunas só de root ficam em `workflow_run` com CHECKs (o índice de ≤1 run ativo por Subject precisa de `state` na mesma tabela). **`external_ref` não foi reservado**: entra por migration aditiva quando houver provider.

Validação: a migration roda em `sqlite3` 3.51 com `-bail`; o fluxo inteiro, de `surt init` ao purge, foi escrito como SQL executável e rodado contra ela (Revisions, snapshots e blobs com hashes reais conferidos); um ciclo completo (Attempt falho, Revision, reject com motivo, aprovação, revalidação, pin, filho com branch bloqueado), 7 recusas esperadas de constraints, purge com `foreign_key_check` e `integrity_check` limpos. Notas de contexto acrescentadas a [Estratégia de recuperação e reconciliação](12-recuperacao-e-reconciliacao.md), [Segurança do cleanup e do ciclo de vida](13-cleanup-e-ciclo-de-vida.md), [Superfície e UX da CLI](14-superficie-e-ux-da-cli.md), [Estrutura do workspace Rust e traits centrais](15-workspace-rust-e-traits.md) e [Estratégia de testes](16-estrategia-de-testes.md). Nenhum ticket novo.
