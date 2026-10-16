# Estratégia de recuperação e reconciliação

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 05, 09, 11, 18

## Question

Como o Surt retoma qualquer workflow — dias depois, após crash, reboot ou kill — sem duplicar trabalho nem confiar em sessões Claude antigas?

Decidir: em que pontos há checkpoint transacional; padrão de intenção → efeito → confirmação para operações externas (git, filesystem, processo); detecção de execuções órfãs (PID + start time? lease expirado?) e o que fazer com seus resultados parciais (worktree suja, commit feito mas não registrado); reconciliação de worktrees e branches contra o SQLite; o que `surt resume` faz passo a passo; como attempt incrementa após falha técnica e limite de retries; erros de rede/rate limit do Claude Code; idempotência de cada operação nativa.

Saída: seção I e a parte de recuperação da seção G.

## Context

Surgiu de [Mecânica de worktree/merge no Git e estados parciais](04-git-worktree-merge-estados-parciais.md): comandos git não são idempotentes por si — a reconciliação é "ler estado → comparar com registro → agir"; usar a tabela de 21 estados parciais do documento como base; regra CAS de `update-ref` (novo → feito, antigo → refazer, outro → parar); worktree `locked initializing`, `index.lock` órfão e marcadores de merge/rebase como casos obrigatórios; risco de `git gc` podar worktrees ausentes após 3 meses em runs retomados dias depois.
