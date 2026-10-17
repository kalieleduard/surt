# Armazenamento de artefatos, congelamento e revalidação

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Assignee: Kaliel
Blocked by: 01

## Question

Onde vivem os artefatos, como são versionados e congelados, e o que acontece quando mudam depois da aprovação?

Decidir: `.surt/features/<feature>/` entra no Git do usuário ou é ignorado?; como Builders em worktrees fora do repo enxergam spec/tasks (cópia congelada em `runtime/runs/<id>/inputs/`? content-addressed por SHA-256?); o que "congelar" significa na aprovação; quando nasce uma nova revisão de artefato (edição humana, regeneração); política de revalidação quando um doc aprovado é editado (bloquear, exigir re-aprovação, ignorar — por quem e quando é detectado); como `surt plan` regenera spec/tasks e o que acontece com revisões anteriores; o que sobra da spec depois do cleanup (só hashes no SQLite? algo no commit do Shipper?); nomenclatura da feature e resolução de colisões.

## Context

Surgiu de [Primitivas multiplataforma](18-primitivas-multiplataforma.md): a normalização de `description` precisa evitar nomes reservados do Windows (CON, NUL, COM1…), caracteres proibidos e ponto/espaço final, e colisões case-insensitive; data local pode falhar (`IndeterminateOffset`) — definir fallback.

De [Glossário do domínio](01-glossario-do-dominio.md): Approval aponta para uma **Artifact Revision** específica e nunca é herdada por uma Revision nova; toda Revision é imutável e tem origem única (Execution ou edição humana); Executions leem Revisions fixadas no início. A política de revalidação decide o que fazer com isso, não se a aprovação "migra".

De [Schema TOML de workflow](06-schema-toml-de-workflow.md): steps leem artefatos **pelo nome, na Revision mais recente** no início da Execution (não "a Revision que o gate aprovou"). Por isso esta política de revalidação é a **única** proteção de "nada não aprovado desce o fluxo". Comportamento já escolhido pelo usuário: um comando que dirige recusa quando a Revision mais recente de um artefato que passou por gate não está aprovada:
```
$ surt plan 20261016-add-login
✖ spec changed after approval (rev 2 not approved)
  review it, then: surt approve 20261016-add-login
```
Falta aqui: como a edição é detectada (hash no início de cada comando?), e o mesmo para edições durante `surt start`.

De [Contrato de agente](07-contrato-de-agente.md): uma Revision de Item List inclui o JSON e todos os arquivos referenciados por `files`; no workflow de feature, inclui um `.md` escrito pelo Task Agent para cada Task. Aprovação cobre o conjunto; editar qualquer `.md` exige nova Revision e reaprovação. Este ticket decide persistência, detecção de edições, vistas locais e revalidação com filhos já iniciados, preservando o snapshot fixado na expansão. Contrato canônico: [F. Agent Contract](../../../docs/architecture/F-agent-contract.md#one-markdown-document-per-task).

## Grilling

### Rodada 1 — armazenamento, edição, Git, inputs, nome

O humano respondeu `concordo com todas` (Q1–Q5): blobs content-addressed no SQLite com publicação atômica e Approval = `revision_id` + hash; Working Copy editável em `.surt/features/<subject>/` com detecção pelo Surt; `.surt/.gitignore` versiona só `config.toml` e `agents/`; diretório por Execution `runtime/executions/<id>/{inputs,delivery}/` materializado dos blobs; nome `YYYYMMDD-<slug>` com as cinco regras (data local com fallback UTC, slug ASCII ≤40, sem nomes reservados pelo prefixo numérico, sufixo `-2`, `--name` quando o slug sai vazio).

### Rodada 2 — detecção, revalidação, conflitos, fim da run

O humano respondeu `aprovo tudo` (Q6–Q11): varredura por SHA-256 (nunca mtime) no início de todo comando da feature e antes de cada Execution, arquivo apagado é recriado; Item List editado é validado como entrega de agente, edição inválida não vira Revision (`surt restore`); Revision aprovada e consumida fica fixada, edição posterior recusada, Driver drena e sai; edição não capturada antes de entrega do agente vira Revision humana e é superada; `approve` varre, aprova a mais recente, `--rev` como trava; SQLite guarda tudo até purge, Working Copy e diretórios de Execution saem no cleanup, nada é commitado.

### Rodada 3 — escopo da Working Copy, termo, ADR

O humano respondeu `concordo com todas` (Q12–Q14): Working Copy só para artefatos com Approval Gate (deduzidos da definição congelada); termo **Pinned Revision** no glossário; ADR 0005 apenas.

## Answer

Resolvido por troca humana nas três rodadas acima. Seção completa em [H. Artifact Storage, Freezing and Revalidation](../../../docs/architecture/H-artifact-storage.md); trade-off em [ADR 0005 — Edits to a pinned Revision are refused, not recorded](../../../docs/adr/0005-edits-to-pinned-revisions-are-refused.md). Glossário ganhou **Working Copy** e **Pinned Revision**; seções A, C (invariante 7), E, F e ADR 0003 apontam para H.

Decisões finais:

- **Armazenamento:** bytes de toda Revision como blobs SHA-256 no `surt.db`; publicar = uma transação; identidade da Revision = hash dos pares `(rel_path, sha256)`; Revisions nunca apagadas enquanto a run existir.
- **Congelar na aprovação:** nada é copiado; Approval aponta `revision_id` + hash. `surt approve` varre, aprova a mais recente, imprime rev + hash; `--rev N` é trava.
- **Working Copy:** só artefatos com gate (`spec`, `manifest` + `tasks/*.md` no `feature`), deduzidos dos `kind = "approval"`; o resto via `surt show`. Agentes nunca leem a Working Copy.
- **Detecção:** SHA-256 contra baseline em todo comando da feature e antes de cada Execution/reescrita; arquivo apagado é recriado; Item List editado passa pela validação de entrega, inválido não vira Revision (`surt restore`).
- **Revalidação:** aprovada e não consumida → edição vira rev N+1 e comandos que dirigem recusam até `approve`. **Pinned** (aprovada e lida como Input; expansão do Per-Item fixa o manifesto) → edição recusada, `approve` recusa, Driver não lança nada novo, deixa terminar e sai; saídas `restore` ou `cancel`.
- **Conflito com entrega de agente:** edição não capturada vira Revision humana antes; a do agente vira a mais recente, sobrescreve e avisa.
- **Inputs de Execution:** `.surt/runtime/executions/<id>/{inputs,delivery}/`, recriado por Attempt a partir dos blobs; `Read` no caminho absoluto; nada copiado para a worktree.
- **Git:** `.surt/.gitignore` gerado (`*`, `!.gitignore`, `!config.toml`, `!agents/**`) — conferido com `git add -A` em repo de teste.
- **Nome:** `YYYYMMDD-<slug>`, data local com fallback UTC, slug ASCII `[a-z0-9-]` ≤40 em fronteira de palavra, sufixo `-N` em colisão, `--name` (mesmas regras, prefixo de data sempre).
- **Fim da run:** SQLite guarda Revisions/Approvals até purge (ticket de cleanup); cleanup remove Working Copy e diretórios de Execution; nada commitado, Shipper não commita a spec.

Validação documental: esboço SQL rodado em `sqlite3`; regras do `.gitignore` conferidas em repo descartável. Notas de contexto acrescentadas a [Modelo de integração Git](09-modelo-de-integracao-git.md), [Schema SQLite](11-schema-sqlite.md), [Segurança do cleanup e do ciclo de vida](13-cleanup-e-ciclo-de-vida.md), [Superfície e UX da CLI](14-superficie-e-ux-da-cli.md) e [Política de logs, retenção e redação](20-politica-de-logs-retencao-e-redacao.md). Nenhum ticket novo: o que sobrou (tabelas, encoding de paths, purge, saídas da CLI) já cabe nesses tickets.
