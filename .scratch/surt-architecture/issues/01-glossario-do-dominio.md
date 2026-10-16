# Glossário do domínio

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Assignee: Kaliel

## Question

Quais são os conceitos centrais do Surt, com nomes canônicos e fronteiras precisas, independentes das tabelas físicas?

Precisa fixar no mínimo: **Workflow Definition** × **Workflow Run**; **Feature** (o assunto `YYYYMMDD-description`) × Workflow Run — uma feature pode ter mais de um run (ex.: re-execução após cancelamento)?; **Step** × **Step Run** (inclui steps filhos gerados por foreach/subworkflow); **Execution**; **Iteration** × **Attempt** (brief: iteration = nova rodada lógica, attempt = nova tentativa após falha técnica) — como se combinam no ciclo Builder↔Reviewer?; **Artifact** × **Artifact Revision** (spec editada pelo humano = nova revisão?); **Approval** (é um Step Run de tipo human approval, um evento, ou ambos?); **Agent** × **Agent Profile**; **Task** (é um conceito do núcleo ou só um artefato que o workflow de feature interpreta?).

Saída: `CONTEXT.md` na raiz + seção C (Domain Model: entidades, relacionamentos, invariantes) em `docs/architecture/`.

## Answer

Resolvido em sessão de grilling. Glossário completo em `CONTEXT.md`; entidades, relações e invariantes em `docs/architecture/C-domain-model.md`; trade-off registrado em `docs/adr/0001-task-cycle-as-child-workflow-run.md`.

Decisões:
- **Workflow Definition × Workflow Run:** o run congela a definição efetiva na criação e nunca a relê; "workflow" sozinho é evitado.
- **Feature × Workflow Run:** o núcleo conhece um **Subject** (nome único que acumula runs ao longo do tempo, no máximo um ativo); Feature é o Subject do workflow de feature, não entidade do núcleo. Re-executar após cancelar = novo run do mesmo Subject.
- **Task:** não é do núcleo — item do manifesto do Task Agent, expandido pelo for-each.
- **Step × Step Run, filhos:** cada Task roda como **Child Workflow Run** do `task-cycle` (ADR 0001), não como Step Runs achatados no run da feature.
- **Execution:** um lançamento concreto (sessão de agente ou Native Operation) de um Step Run; 0..N por Step Run, no máximo uma rodando; Approval Gate tem zero.
- **Iteration × Attempt:** Iteration = nova rodada lógica por decisão (ex.: review reprovado) que cria **novos Step Runs**; Attempt = nova Execution do mesmo Step Run por falha técnica. Step Run terminado nunca é reaberto.
- **Artifact × Artifact Revision:** Revision imutável, origem única (uma Execution ou uma edição humana); Executions leem Revisions fixadas no início (Inputs) e produzem novas (Outputs).
- **Approval:** **Approval Gate** (step que pausa o run) + **Approval** (decisão registrada sobre uma Revision específica); nova Revision não herda Approval.
- **Agent × Agent Profile × Agent Runtime:** papel × forma concreta de rodar × adaptador (Claude Code, Fake Runtime). "Provider" reservado para integrações externas.
- **Papéis do workflow de feature:** Spec Agent, Task Agent, Builder, Reviewer, Shipper.

Requisito surgido: N features aprovadas devem poder rodar simultaneamente no mesmo repo — registrado em [Modelo de processo e controle de concorrência](05-modelo-de-processo-e-concorrencia.md).
