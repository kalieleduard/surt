# Mecânica de worktree/merge no Git e estados parciais

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: research
Status: claimed (research subagent)

## Question

Que estados externos parciais o Git pode deixar quando o Surt é interrompido no meio de uma operação, e como detectá-los e reconciliá-los de forma idempotente?

Levantar: `git worktree add/remove/prune/lock/repair` (com worktrees fora do repo, ex.: `~/.surt/worktrees/...`), o que acontece se o diretório da worktree some ou o metadado em `.git/worktrees/` fica órfão; branch já existente/checada em outra worktree; `index.lock` residual; merge/rebase/cherry-pick interrompidos (`MERGE_HEAD`, `rebase-merge/`) e como abortar/detectar; merges sem worktree (`git merge-tree --write-tree` + `commit-tree` + `update-ref` com old-value) como alternativa atômica; `update-ref` com verificação de valor antigo para idempotência; obter diff e SHA exatos para vincular review a commit; comportamento no Windows (paths longos, `core.longpaths`, symlinks, file locking que impede remoção); versão mínima do Git necessária; `git2`/`gix` × invocar o binário `git`.

Findings: `docs/research/git-worktree-merge.md` na branch `research/git-worktree-merge`.
