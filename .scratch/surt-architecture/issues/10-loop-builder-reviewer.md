# Máquina de estados do loop Builder ↔ Reviewer

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 06, 07, 09

## Question

Como o ciclo Builder ↔ Reviewer de uma Task é modelado como máquina de estados (dentro do subworkflow) sem ciclos no DAG?

Decidir: estados e transições do task-cycle; como iteration e attempt incrementam; limite de iterações e o que acontece ao estourar (Task `BLOCKED`? workflow `BLOCKED`? intervenção humana?); como findings do Reviewer viram artefato de feedback consumido pelo próximo Builder; escalonamento de perfil entre iterações; o que acontece se o Builder não produz commit ou o Reviewer devolve saída inválida; review de mudanças triviais após conflito de integração.

Saída: parte da seção G.
