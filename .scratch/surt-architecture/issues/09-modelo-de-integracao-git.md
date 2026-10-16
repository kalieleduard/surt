# Modelo de integração Git

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 01, 04

## Question

Como o código flui das branches de Task até a branch entregue pelo Shipper, garantindo que nada não-revisado entre?

Decidir: nome e base da branch de integração por feature; de qual commit cada branch de Task parte (integração no momento do start?); quem integra uma Task aprovada (operação nativa determinística × agente) e quando; o que acontece em conflito (nova iteração do Builder rebaseada? re-review obrigatório?); como o review fica vinculado ao SHA exato (e o que invalida a aprovação); quem roda build/lint/test (comandos declarados em config e executados como step nativo? resultados viram artefato para o Reviewer?); o papel exato do Shipper (agente × nativo × híbrido) e o que ele entrega (branch integrada final, sem merge na main); layout de `~/.surt/worktrees/<project>/<feature>/<task>/` e identificação de projeto.

## Context

Surgiu de [Mecânica de worktree/merge no Git e estados parciais](04-git-worktree-merge-estados-parciais.md) — decidir aqui: integração via `merge-tree` + `commit-tree` + `update-ref` (sem worktree) × merge dentro de worktree; versão mínima do Git exigida (≥2.38/2.40/2.43); CLI `git` × libgit2/gix (a pesquisa aponta CLI); nunca mover via `update-ref` uma branch checada em outra worktree; vínculo do review ao SHA e política de `core.autocrlf` no Windows; configs que o Surt impõe ou checa (`core.longpaths`, `core.lockfilePid`).

De [Modelo de processo e controle de concorrência](05-modelo-de-processo-e-concorrencia.md): as Native Operations do Surt que mexem no estado compartilhado do repo (worktree add/remove, criação de branch, `update-ref`, `merge-tree`/`commit-tree`) rodam serializadas sob `.surt/runtime/locks/git.lock`, com retry com backoff para locks do Git criados por terceiros; commits do Builder na própria worktree ficam fora desse lock.
