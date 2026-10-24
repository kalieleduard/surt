<sqlite_rules>
  <scope>
    - Apply to changed SQLite schemas, queries, connections, and migrations.
    - Respect deployed versions; ignore unrelated legacy code.
  </scope>

  <priorities>
    Integrity/durability/compatibility > simplicity > speed.
    Report trade-offs; never sacrifice integrity for speed.
  </priorities>

  <schema>
    - Prefer INTEGER PRIMARY KEY; avoid AUTOINCREMENT unless ID reuse is forbidden.
    - Prefer STRICT if supported; enforce NOT NULL, UNIQUE, CHECK, and FKs.
    - Use FKs for 1:N and junctions for N:N; do not hide relations in JSON.
    - Use CASCADE only for owned children; specify deletion behavior.
    - Use UTC Unix seconds (INTEGER); booleans as INTEGER CHECK (value IN (0,1)).
  </schema>

  <connections>
    - Set foreign_keys = ON per connection outside transactions; set busy_timeout.
    - Prefer WAL on local disks; expect one writer and short transactions.
    - Use synchronous = FULL for power-loss durability, else consider NORMAL.
    - Bind every value as a query parameter; never build SQL by string formatting.
    - Use transactions for related writes.
  </connections>

  <migrations>
    <why>Every project's .surt/surt.db outlives the binary that created it; an edited migration leaves deployed databases on a schema the code no longer expects.</why>
    - Never edit a migration that has shipped; add a new one.
    - Make each migration preserve existing data and test it against a database at the previous version.
  </migrations>

  <performance>
    - Index real WHERE/JOIN/ORDER BY patterns and queried FKs.
    - Avoid redundant indexes; use EXPLAIN QUERY PLAN and measurements.
    - Use PRAGMA optimize as needed; do not denormalize without evidence.
  </performance>

  <reliability>
    - Test data preservation and recovery.
    - Back up live DBs with Backup API or VACUUM INTO, not only .db.
    - Validate with integrity_check and foreign_key_check.
  </reliability>

  <report>
    Report with the three-line format of clean-code.md.
    Example exception: WAL; src/db.rs:open; network share; rollback journal retained.
  </report>
</sqlite_rules>
