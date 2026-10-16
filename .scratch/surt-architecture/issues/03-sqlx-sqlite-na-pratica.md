# SQLx + SQLite na prática

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: research
Status: resolved

## Question

Quais são as restrições e práticas recomendadas para usar SQLx com SQLite embarcado num binário CLI distribuível?

Levantar: armazenamento de UUIDv7 (BLOB 16 bytes × TEXT) e suporte do SQLx/`uuid` a cada um, ordenação e índices; `sqlx::migrate!` embutido no binário e comportamento com migrations já aplicadas/checksum; macros `query!`/`query_as!` exigem `DATABASE_URL`/modo offline (`.sqlx`) — custo para o build e alternativas; pragmas por conexão (`foreign_keys`, `journal_mode=WAL`, `busy_timeout`, `synchronous`) via `SqliteConnectOptions`; concorrência de escrita (um writer) com Tokio e vários processos `surt` simultâneos; `BEGIN IMMEDIATE` e transações; timestamps UTC (TEXT ISO-8601 × INTEGER) e funções disponíveis; CHECK constraints, partial indexes, `STRICT` tables, generated columns, JSON1; WAL em filesystems de rede/Windows; tamanho do binário e bundling do SQLite (`libsqlite3-sys` bundled).

Findings: `docs/research/sqlx-sqlite.md` na branch `research/sqlx-sqlite`.

## Answer

Achados completos: `docs/research/sqlx-sqlite.md` na branch `research/sqlx-sqlite` (commit `6d3cf59`; sqlx **0.9.0**, MSRV 1.94; repo migrou para `transact-rs/sqlx`).

Fatos que restringem o design (não são decisões):
- Feature `sqlite` empacota SQLite estaticamente (exige toolchain C no build); faixa 3.46.0–3.51.3, fixável via `Cargo.lock`. STRICT, RETURNING, generated columns, partial indexes, JSON e `unixepoch`/`subsec` disponíveis.
- `uuid::Uuid` vira BLOB(16); TEXT exige `fmt::Hyphenated`. Ambos ordenam por tempo para UUIDv7. Ordem do `now_v7()` só é garantida dentro de um processo.
- sqlx liga `foreign_keys` e `busy_timeout=5s` por padrão; **não** liga WAL nem `synchronous`; `create_if_missing=false`. A primeira troca para WAL exige lock exclusivo e pode colidir entre processos.
- `begin()` é DEFERRED → read-then-write pode falhar na hora com SQLITE_BUSY/BUSY_SNAPSHOT ignorando busy timeout; `begin_with("BEGIN IMMEDIATE")` existe; BUSY chega como `ErrorKind::Other` (retry deve casar `code()`).
- Lock de migração no SQLite é no-op → dois processos migrando ao mesmo tempo dependem só do locking do SQLite (provável erro em um; não testado).
- Checksum de migration é SHA-384 dos bytes → CRLF × LF quebra (`VersionMismatch`; caso real no openai/codex). Binário velho com DB novo falha (`VersionMissing`). Migrations rodam em transação onde `PRAGMA foreign_keys=OFF` não funciona — rebuild de tabela exige `-- no-transaction`/`defer_foreign_keys`.
- Timestamps: chrono grava `...T...+00:00` com frações variáveis; `CURRENT_TIMESTAMP` grava `YYYY-MM-DD HH:MM:SS` — não comparam como texto. INTEGER é lido como segundos (ms exige `i64` manual). `ADD COLUMN` não aceita default expressão.
- `query!` exige `.sqlx` commitado + `cargo sqlx prepare --check` em CI (ou DB no build); `DATABASE_URL` perdido vence `.sqlx`. Em tabelas STRICT as macros inferem `Vec<u8>`/`String`/`i64` e pedem overrides. `query_as::<_, T>` + `FromRow` em runtime evita acoplamento de build.
- WAL: mesmo host, diretório gravável, `-wal`/`-shm` acompanham o DB; WAL cresce sem limite se checkpoints forem starvados.
- Um writer por vez; cada conexão do pool é uma thread. `#[sqlx::test]` dá DB em arquivo novo por teste com migrations; in-memory shared-cache não reproduz WAL/multiprocesso.

Abertos: tamanho de binário/tempo de compilação (medir), migração concorrente real (experimento), formato do `time` (`Z` × `+00:00`).
