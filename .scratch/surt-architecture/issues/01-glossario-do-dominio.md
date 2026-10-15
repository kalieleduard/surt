# Glossário do domínio

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open

## Question

Quais são os conceitos centrais do Surt, com nomes canônicos e fronteiras precisas, independentes das tabelas físicas?

Precisa fixar no mínimo: **Workflow Definition** × **Workflow Run**; **Feature** (o assunto `YYYYMMDD-description`) × Workflow Run — uma feature pode ter mais de um run (ex.: re-execução após cancelamento)?; **Step** × **Step Run** (inclui steps filhos gerados por foreach/subworkflow); **Execution**; **Iteration** × **Attempt** (brief: iteration = nova rodada lógica, attempt = nova tentativa após falha técnica) — como se combinam no ciclo Builder↔Reviewer?; **Artifact** × **Artifact Revision** (spec editada pelo humano = nova revisão?); **Approval** (é um Step Run de tipo human approval, um evento, ou ambos?); **Agent** × **Agent Profile**; **Task** (é um conceito do núcleo ou só um artefato que o workflow de feature interpreta?).

Saída: `CONTEXT.md` na raiz + seção C (Domain Model: entidades, relacionamentos, invariantes) em `docs/architecture/`.
