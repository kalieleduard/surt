# Segurança do cleanup e do ciclo de vida

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 08, 12, 18

## Question

Como o cleanup remove exatamente o que deve — e nada mais — de forma idempotente, auditável e recuperável?

Decidir: máquina de estados do cleanup (`on_success`, `on_delete`); que estados bloqueiam remoção; o que é removido (feature dir, inputs congelados, worktrees, branches de task?) e o que é preservado; proteção contra path traversal, symlinks/junctions (Windows), arquivos fora da raiz esperada; ordem de remoção e retomada após interrupção no meio; o que `surt workflow delete` faz no SQLite (soft delete?) e no disco; conteúdo sensível reproduzido em `stream.jsonl`/logs.
