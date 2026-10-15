# Modelo de integração Git

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 01, 04

## Question

Como o código flui das branches de Task até a branch entregue pelo Shipper, garantindo que nada não-revisado entre?

Decidir: nome e base da branch de integração por feature; de qual commit cada branch de Task parte (integração no momento do start?); quem integra uma Task aprovada (operação nativa determinística × agente) e quando; o que acontece em conflito (nova iteração do Builder rebaseada? re-review obrigatório?); como o review fica vinculado ao SHA exato (e o que invalida a aprovação); quem roda build/lint/test (comandos declarados em config e executados como step nativo? resultados viram artefato para o Reviewer?); o papel exato do Shipper (agente × nativo × híbrido) e o que ele entrega (branch integrada final, sem merge na main); layout de `~/.surt/worktrees/<project>/<feature>/<task>/` e identificação de projeto.
