# Máquina de estados do loop Builder ↔ Reviewer

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 06, 07, 09

## Question

Como o ciclo Builder ↔ Reviewer de uma Task é modelado como máquina de estados (dentro do subworkflow) sem ciclos no DAG?

Decidir: estados e transições do task-cycle; como iteration e attempt incrementam; limite de iterações e o que acontece ao estourar (Task `BLOCKED`? workflow `BLOCKED`? intervenção humana?); como findings do Reviewer viram artefato de feedback consumido pelo próximo Builder; escalonamento de perfil entre iterações; o que acontece se o Builder não produz commit ou o Reviewer devolve saída inválida; review de mudanças triviais após conflito de integração.

Saída: parte da seção G.

## Context

De [Glossário do domínio](01-glossario-do-dominio.md), já decidido: o task-cycle é um Child Workflow Run por Task; **Iteration** = nova rodada por decisão (review reprovado), cria novos Step Runs `build`/`review` no run filho; **Attempt** = nova Execution do mesmo Step Run por falha técnica. Falta aqui: estados, transições, limites de cada um e o que acontece ao estourar.

De [Modelo de processo e controle de concorrência](05-modelo-de-processo-e-concorrencia.md): a máquina de estados precisa nomear: Execution **interrompida** (Ctrl-C/Interruption → novo Attempt na retomada), Execution **na fila** esperando vaga de agente, **cancel request** pendente num run com Driver vivo, e o estado terminal de **Cancellation**. Decidir aqui se uma Interruption conta para o limite de Attempts.
