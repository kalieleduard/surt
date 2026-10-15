# SQLx + SQLite na prática

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: research
Status: claimed (research subagent)

## Question

Quais são as restrições e práticas recomendadas para usar SQLx com SQLite embarcado num binário CLI distribuível?

Levantar: armazenamento de UUIDv7 (BLOB 16 bytes × TEXT) e suporte do SQLx/`uuid` a cada um, ordenação e índices; `sqlx::migrate!` embutido no binário e comportamento com migrations já aplicadas/checksum; macros `query!`/`query_as!` exigem `DATABASE_URL`/modo offline (`.sqlx`) — custo para o build e alternativas; pragmas por conexão (`foreign_keys`, `journal_mode=WAL`, `busy_timeout`, `synchronous`) via `SqliteConnectOptions`; concorrência de escrita (um writer) com Tokio e vários processos `surt` simultâneos; `BEGIN IMMEDIATE` e transações; timestamps UTC (TEXT ISO-8601 × INTEGER) e funções disponíveis; CHECK constraints, partial indexes, `STRICT` tables, generated columns, JSON1; WAL em filesystems de rede/Windows; tamanho do binário e bundling do SQLite (`libsqlite3-sys` bundled).

Findings: `docs/research/sqlx-sqlite.md` na branch `research/sqlx-sqlite`.
