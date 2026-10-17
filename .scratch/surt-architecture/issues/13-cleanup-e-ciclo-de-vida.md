# Cleanup, `surt delete` e logs

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 08, 12, 18

## Question

Quando um run termina (ou o usuário apaga a Feature), o que o Surt remove, o que preserva, e em que ordem — de forma que rodar de novo depois de uma queda termine o serviço?

Decidir, só para a v1:
- **`on_success = "clean"`**: destino de cada coisa que um run deixa — Working Copy (`.surt/features/<subject>/`), `.surt/runtime/executions/<id>/`, worktrees das Tasks, Task Branches, Salvage Refs. A Integration Branch é o entregável e fica.
- **Run `cancelled`**: mesmo destino de `succeeded` ou mantém tudo para inspeção?
- **`surt delete <feature>`**: purge do Subject — ordem disco → Git → SQLite (o caminho da worktree vive em `run_branch.worktree_path`, então a linha sai por último) e o que acontece se cair no meio (rodar `delete` de novo termina).
- **Logs** (`stream.jsonl`, `stderr.log` por Execution): ficam crus junto da Execution e saem no cleanup? Que metadados mínimos ficam no SQLite para `surt inspect` funcionar depois que o stream sumiu.

Fora da v1 (não decidir aqui): redação de segredos nos logs, opt-in de logs completos, "arquivar sem apagar", limpeza de diretórios `.orphaned-*`, commits sem ref (o `git gc` resolve), proteção contra junctions/`\\?\` além do que `remove_dir_all` já dá.

## Context

- **Estados:** só `succeeded` e `cancelled` são terminais; run `blocked` é ativo e o cleanup não toca ([G. State Machines](../../../docs/architecture/G-state-machines.md)).
- **Revisions ficam no SQLite** (blobs + Approvals) até um purge explícito; a Working Copy é só vista local e `surt show` lê Revisions depois que ela some ([H. Artifact Storage](../../../docs/architecture/H-artifact-storage.md)).
- **Purge** = um `DELETE FROM subject` em transação (cascata) + GC de snapshots/blobs sem referência; não existe `deleted_at` ([ADR 0009](../../../docs/adr/0009-history-is-purged-not-soft-deleted.md), [D](../../../docs/architecture/D-database.md#deleting-history)).
- **Git:** worktrees em `~/.surt/worktrees/<project-id>/<subject>/<key>/`, criadas com `--lock` (destravar antes de `remove`); remoção passa por `git.lock`; cleanup nunca mexe na Base nem na worktree do usuário ([A](../../../docs/architecture/A-git-integration.md), [I](../../../docs/architecture/I-recovery.md)).
- **CLI:** `surt delete` confirma no TTY mostrando o impacto, `--yes` pula, sem TTY e sem `--yes` recusa com exit `3`; leitura nunca dispara cleanup; `surt logs` lê o stream do disco e, se ele não existe mais, aponta para `inspect` ([J](../../../docs/architecture/J-cli.md)).
- **TOML:** o único controle por workflow é `[cleanup] on_success = "clean" | "keep"` (default `clean`).

Absorveu o antigo ticket 20 (Política de logs, retenção e redação): para a v1 basta decidir onde o log fica e quando sai.
