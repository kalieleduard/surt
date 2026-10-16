# Segurança do cleanup e do ciclo de vida

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 08, 12, 18

## Question

Como o cleanup remove exatamente o que deve — e nada mais — de forma idempotente, auditável e recuperável?

Decidir: máquina de estados do cleanup (`on_success`, `on_delete`); que estados bloqueiam remoção; o que é removido (feature dir, inputs congelados, worktrees, branches de task?) e o que é preservado; proteção contra path traversal, symlinks/junctions (Windows), arquivos fora da raiz esperada; ordem de remoção e retomada após interrupção no meio; o que `surt workflow delete` faz no SQLite (soft delete?) e no disco; conteúdo sensível reproduzido em `stream.jsonl`/logs.

## Context

Surgiu de [Primitivas multiplataforma](18-primitivas-multiplataforma.md): `remove_dir_all` não segue links abaixo da raiz mas não é idempotente nem protege contra troca de ancestral pós-checagem; contenção de path precisa lidar com `\\?\`, case-insensitivity e junctions.

De [Schema TOML de workflow](06-schema-toml-de-workflow.md): o único controle por workflow no TOML é `[cleanup] on_success = "clean" | "keep"` (default `clean`); este ticket decide o que "clean" remove e a política global/`on_delete`.
