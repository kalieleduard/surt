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

De [Modelo de processo e controle de concorrência](05-modelo-de-processo-e-concorrencia.md), já decidido: órfã = Execution em estado de execução cujo lock do root run está livre; órfãos **nunca são adotados** — o comando que dirige mata o grupo sobrevivente (identidade pid + start time + boot id) e cria novo Attempt. Não há `surt resume`: reexecutar `surt feature`/`plan`/`start` retoma. O primeiro Ctrl-C deixa a Native Operation em andamento terminar; só o segundo Ctrl-C (ou kill -9/crash) deixa estado parcial para reconciliar. Falta aqui: o que fazer com a worktree suja/commit não registrado que o órfão ou a Interruption deixaram.

De [Modelo de integração Git](09-modelo-de-integracao-git.md) ([A. Git Integration](../../../docs/architecture/A-git-integration.md)): estados parciais a reconciliar por Operation: `git.create_integration_branch` (branch existe?), `git.create_task_worktree` (branch sem worktree, worktree `locked initializing`), `git.integrate` (ref = M → feito; = T → refazer; outro → parar; worktree da Task deixada em `detach M` no meio do re-check; commit M sem ref é lixo inofensivo), `git.merge_into_base` (`merge --ff-only` interrompido na worktree do usuário). Integration Branch checada em outra worktree = falha técnica, retentada no próximo comando; ponta divergente da gravada = **para sem adotar**, com a instrução `git branch -f`. Decidir aqui o estado do run nesse caso e como a retomada confirma que a ponta foi restaurada.
