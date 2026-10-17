# Segurança do cleanup e do ciclo de vida

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 08, 12, 18, 20

## Question

Como o cleanup remove exatamente o que deve — e nada mais — de forma idempotente, auditável e recuperável?

Decidir: máquina de estados do cleanup (`on_success`, `on_delete`); que estados bloqueiam remoção; o que é removido (feature dir, inputs congelados, worktrees, branches de task?) e o que é preservado; proteção contra path traversal, symlinks/junctions (Windows), arquivos fora da raiz esperada; ordem de remoção e retomada após interrupção no meio; o que `surt workflow delete` faz no SQLite (soft delete?) e no disco; conteúdo sensível reproduzido em `stream.jsonl`/logs.

## Context

De [Contrato de agente](07-contrato-de-agente.md): manifesto e `.md` por Task formam uma única Revision aprovada. O cleanup deve preservar as garantias do armazenamento de Revisions, sem tratar vistas locais como a única cópia de documentos aprovados. A retenção/redação de streams e stderr é decidida em [Política de logs, retenção e redação](20-politica-de-logs-retencao-e-redacao.md) antes de este ticket fixar o que remove.

Surgiu de [Primitivas multiplataforma](18-primitivas-multiplataforma.md): `remove_dir_all` não segue links abaixo da raiz mas não é idempotente nem protege contra troca de ancestral pós-checagem; contenção de path precisa lidar com `\\?\`, case-insensitivity e junctions.

De [Schema TOML de workflow](06-schema-toml-de-workflow.md): o único controle por workflow no TOML é `[cleanup] on_success = "clean" | "keep"` (default `clean`); este ticket decide o que "clean" remove e a política global/`on_delete`.

De [Armazenamento de artefatos, congelamento e revalidação](08-armazenamento-congelamento-revalidacao.md): o cleanup de uma run terminal remove `.surt/features/<subject>/` (Working Copy) e `.surt/runtime/executions/<id>/`; Revisions, blobs e Approvals ficam no SQLite até um **purge** explícito, cuja política (quando, comando, confirmação) é decidida aqui. Ver [H. Artifact Storage](../../../docs/architecture/H-artifact-storage.md).

De [Modelo de integração Git](09-modelo-de-integracao-git.md) ([A. Git Integration](../../../docs/architecture/A-git-integration.md)): o que um run terminal deixa no Git: Integration Branch `surt/<subject>[.N]` (o entregável, que não deve ser apagado em `on_success`), Task Branches `surt/task/<subject>[.N]/<key>` e worktrees em `~/.surt/worktrees/<project-id>/<subject>[.N]/<key>/` (caminho absoluto no SQLite), além de commits `M` sem ref de re-checks reprovados. Decidir o destino de cada um em `on_success`, `on_delete` e cancelado; remover worktree passa por `git.lock`. Se o `surt.db` for apagado, o project id muda e as worktrees antigas ficam órfãs.

De [Máquina de estados do loop Builder ↔ Reviewer](10-loop-builder-reviewer.md) ([G. State Machines](../../../docs/architecture/G-state-machines.md)): não existe run `failed`: um run `blocked` continua `active` indefinidamente e o cleanup deve tratá-lo como ativo; só `succeeded` e `cancelled` são terminais. Filhos não são cancelados individualmente no M1.

De [Schema SQLite](11-schema-sqlite.md) ([D. Database Schema](../../../docs/architecture/D-database.md)#deleting-history): não existe `deleted_at`. Purge = um `DELETE FROM subject` em transação (cascata por runs, Step Runs, Executions, Revisions, Approvals, `run_branch` e `event`) seguido de GC de snapshots sem `run_snapshot` e blobs sem referência ([ADR 0009](../../../docs/adr/0009-history-is-purged-not-soft-deleted.md)). Decidir aqui quando/como o purge é oferecido e confirmado, e a ordem disco → Git → SQLite (o caminho das worktrees está em `run_branch.worktree_path`; apagar a linha antes de remover a worktree perde esse caminho). "Arquivar sem apagar", se quiser, é coluna nova em `subject`.
