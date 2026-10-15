# Schema SQLite

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 01, 03, 05, 06, 08

## Question

Qual é o schema físico completo (7 tabelas) que serve a qualquer workflow e preserva integridade referencial sem JSON para relações críticas?

Decidir: colunas, tipos, PK UUIDv7, FKs e `ON DELETE`; o que `deleted_at` significa em cada tabela (inclusive `events`, que deveria ser append-only); constraints de status (CHECK) e unicidade (ex.: uma execution ativa por step; `(workflow_run_id, logical_key)`; `(step_run_id, iteration, attempt)`); `step_dependencies` e `execution_artifacts` (papel input/output); revisões e hashes de artefatos; onde moram approval, lease/heartbeat, `external_ref`; índices para o scheduler e para `surt status`; estratégia de migrations; o que fica em JSON (snapshots, payloads) e o que não pode ficar.

Saída: seção D com SQL que roda em `sqlite3`, ERD Mermaid e migration inicial.
