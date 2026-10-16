# Mecânica de worktree/merge no Git e estados parciais

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: research
Status: resolved

## Question

Que estados externos parciais o Git pode deixar quando o Surt é interrompido no meio de uma operação, e como detectá-los e reconciliá-los de forma idempotente?

Levantar: `git worktree add/remove/prune/lock/repair` (com worktrees fora do repo, ex.: `~/.surt/worktrees/...`), o que acontece se o diretório da worktree some ou o metadado em `.git/worktrees/` fica órfão; branch já existente/checada em outra worktree; `index.lock` residual; merge/rebase/cherry-pick interrompidos (`MERGE_HEAD`, `rebase-merge/`) e como abortar/detectar; merges sem worktree (`git merge-tree --write-tree` + `commit-tree` + `update-ref` com old-value) como alternativa atômica; `update-ref` com verificação de valor antigo para idempotência; obter diff e SHA exatos para vincular review a commit; comportamento no Windows (paths longos, `core.longpaths`, symlinks, file locking que impede remoção); versão mínima do Git necessária; `git2`/`gix` × invocar o binário `git`.

Findings: `docs/research/git-worktree-merge.md` na branch `research/git-worktree-merge`.

## Answer

Achados completos: `docs/research/git-worktree-merge.md` na branch `research/git-worktree-merge` (commit `9b2d8d1`; testado com git 2.56.0 no macOS; Linux/Windows só por documentação). Inclui tabela de 21 linhas "estado parcial → detecção → reconciliação segura".

Fatos que restringem o design (não são decisões):
- `worktree add/remove/lock` e todo `--abort` **falham se reexecutados após sucesso** — idempotência tem de vir do Surt ler o estado (`worktree list --porcelain -z`, `rev-parse -q --verify`) e comparar com o SQLite.
- `worktree add -b` não é atômico: Ctrl-C deixa a branch sem worktree; kill duro deixa worktree `locked initializing` sem index e com `index.lock` (`remove -f -f` resolve; `prune` não). Worktree movida vira `prunable`; `git gc` poda worktrees ausentes e destravadas após 3 meses; metadado `.git/worktrees/<id>` apagado não tem reparo.
- Git nunca remove locks velhos; `index.lock` e marcadores de merge/rebase são por worktree, locks de ref são do repo inteiro. `core.lockfilePid` (2.56, off por padrão) registra o PID dono. `status --no-optional-locks` evita que o polling do Surt pegue o lock.
- `status --porcelain=v2` **não sinaliza** merge/rebase em andamento — detectar por marcadores (`MERGE_HEAD`, `rebase-merge/`, `CHERRY_PICK_HEAD`, `sequencer/` …) via `rev-parse --git-path`.
- Integração sem worktree é viável e reexecutável: `merge-tree --write-tree` (0 limpo, 1 conflito) → `commit-tree` com datas fixas (SHA determinístico) → `update-ref --stdin` transacional. Exige Git ≥2.38 (≥2.40 para `--merge-base`, ≥2.43 para `-X`). Em conflito ainda devolve uma tree com marcadores — nunca commitar.
- CAS de `update-ref`: reexecução após sucesso falha; regra de recuperação: ref = novo → feito; = antigo → refazer; outro → parar. `update-ref` move branch checada em outra worktree sem reclamar (desincroniza a worktree); `symref-update HEAD` sem `no-deref` corrompe branches.
- Vínculo de review: o SHA do commit cobre conteúdo + ancestralidade; `diff A...B` = `diff $(merge-base) B`. No Windows, o que é commitado depende de `core.autocrlf`.
- libgit2 e gix não substituem a CLI (sem `worktree repair`; merge do libgit2 não é ort; `git2::Transaction` não é atômico; gix em desenvolvimento inicial). Git for Windows vem com long paths e symlinks desligados.
