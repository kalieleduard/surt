# Estratégia de recuperação e reconciliação

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 05, 09, 11, 18

## Question

Como o Surt retoma qualquer workflow — dias depois, após crash, reboot ou kill — sem duplicar trabalho nem confiar em sessões Claude antigas?

Decidir: em que pontos há checkpoint transacional; padrão de intenção → efeito → confirmação para operações externas (git, filesystem, processo); detecção de execuções órfãs (PID + start time? lease expirado?) e o que fazer com seus resultados parciais (worktree suja, commit feito mas não registrado); reconciliação de worktrees e branches contra o SQLite; o que `surt resume` faz passo a passo; como attempt incrementa após falha técnica e limite de retries; erros de rede/rate limit do Claude Code; idempotência de cada operação nativa.

Saída: seção I e a parte de recuperação da seção G.
