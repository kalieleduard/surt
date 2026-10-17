# Cleanup, `surt delete` e logs

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Blocked by: 08, 12, 18
Assignee: Kaliel

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

## Answer

Resolvido por troca humana em duas rodadas (9 perguntas, todas aceitas com a recomendação). Seção completa em [I. Cleanup, Delete and Logs](../../../docs/architecture/I-cleanup.md). Seções A (Git e processo), D, E, F, H, I-recovery e J atualizadas para apontar para ela. `CONTEXT.md` ganha **Cleanup** e **Purge**. Sem ADR novo: nenhuma decisão é difícil de reverter (nada muda o schema além de uma coluna nullable).

Decisões finais:

- **Quando:** o cleanup roda no mesmo comando que grava o estado terminal, ainda com o lock do run (Driver no `succeeded`; Driver ou `surt cancel` no `cancelled`). Não é Step. Nova coluna `workflow_run.cleaned_at` (só root, só terminal). Terminal + política de limpar + `cleaned_at` NULL = **cleanup pending**.
- **Falha:** cada item é idempotente. Uma falha (ex.: arquivo em uso no Windows) não para os outros, não muda o exit code, grava `cleanup.incomplete` e avisa com `retry:`. Quem completa: qualquer comando que pega o lock daquele run (`feature`/`plan`/`start`/`cancel`/`delete`), nunca leitura, nunca outro Subject. Avisos finais (não deixam pendente): edição não registrada, Task Branch movida por terceiros, `worktree_path` que não é mais worktree do repo.
- **`succeeded` + `clean`:** remove a Working Copy (salvo edições), `runtime/executions/<id>/`, as worktrees (`unlock` + `remove --force`; ignorados saem junto), as Task Branches (com CAS no `tip_sha`) e as Salvage Refs. Ficam a Integration Branch, o SQLite e os logs. Com `keep` nada sai, o `status` mostra `kept` e só o `delete` limpa.
- **`cancelled`:** remove o que pesa em disco (Working Copy, diretórios de Execution e worktrees, salvando antes as sobras em Salvage Ref) e mantém todas as refs (Integration Branch, Task Branches, Salvage Refs) até o `surt delete`. O `on_success` não se aplica.
- **Logs:** em `.surt/logs/<subject>/<execution-id>/{stream.jsonl,stderr.log,output.log}` (`output.log` = saída completa dos Checks). Sobrevivem a todo cleanup e saem só no `delete`. `runtime/executions/<id>/` fica só com `inputs/` e `delivery/`. Para o `inspect` sem stream, `runtime_info` ganha `exit_code` e `num_turns` (JSON, sem coluna nova).
- **Edição não registrada na Working Copy:** o cleanup nunca apaga; mantém o arquivo e avisa. `--new-run` recusa com exit `3` enquanto ela existir. O `delete` remove e lista.
- **`surt delete`:** recusa com exit `3` se há run ativo (`fix: surt cancel`). Mantém a Integration Branch de run `succeeded` (pode ter PR aberto) e apaga a de run `cancelled`. Task Branches e Salvage Refs saem sem CAS (a confirmação as listou). Nunca toca a Base nem a worktree do usuário.
- **Ordem do `delete`:** `try_lock` de todo root run do Subject → worktrees, refs (um `update-ref --stdin`) e diretórios → `BEGIN IMMEDIATE`, que relê os runs e recusa se apareceu um `--new-run` concorrente, depois o purge → solta e remove os lock files. Uma falha no passo de Git/disco pula o purge e sai com `1` + `retry:`. Rodar de novo termina o serviço. Sem marcador `deleting`.
- **Fora da v1:** remover a worktree de cada Task assim que ela é integrada (há um único cleanup, no fim do root run).

Mudança no schema (migration 0001 ainda não lançada, editada): `workflow_run.cleaned_at` com `CHECK (cleaned_at IS NULL OR (parent_step_run_id IS NULL AND state <> 'active'))`. Event kinds novos: `cleanup.finished`, `cleanup.incomplete`. Validação: a migration roda em `sqlite3` 3.51 com `-bail`; o CHECK aceita run terminal limpo e recusa `cleaned_at` em run ativo; o purge em cascata remove tudo. Fato verificado no Git 2.56: `worktree remove` aceita worktree só com ignorados e apaga o diretório; não rastreados exigem `--force`; worktree travada exige `unlock`.

Notas de contexto acrescentadas a [Estrutura do workspace Rust e traits centrais](15-workspace-rust-e-traits.md), [Estratégia de testes](16-estrategia-de-testes.md) e [Roadmap e Milestone 1](17-roadmap-e-milestone-1.md). Nenhum ticket novo.
