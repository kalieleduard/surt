# Armazenamento de artefatos, congelamento e revalidação

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 01

## Question

Onde vivem os artefatos, como são versionados e congelados, e o que acontece quando mudam depois da aprovação?

Decidir: `.surt/features/<feature>/` entra no Git do usuário ou é ignorado?; como Builders em worktrees fora do repo enxergam spec/tasks (cópia congelada em `runtime/runs/<id>/inputs/`? content-addressed por SHA-256?); o que "congelar" significa na aprovação; quando nasce uma nova revisão de artefato (edição humana, regeneração); política de revalidação quando um doc aprovado é editado (bloquear, exigir re-aprovação, ignorar — por quem e quando é detectado); como `surt plan` regenera spec/tasks e o que acontece com revisões anteriores; o que sobra da spec depois do cleanup (só hashes no SQLite? algo no commit do Shipper?); nomenclatura da feature e resolução de colisões.

## Context

Surgiu de [Primitivas multiplataforma](18-primitivas-multiplataforma.md): a normalização de `description` precisa evitar nomes reservados do Windows (CON, NUL, COM1…), caracteres proibidos e ponto/espaço final, e colisões case-insensitive; data local pode falhar (`IndeterminateOffset`) — definir fallback.

De [Glossário do domínio](01-glossario-do-dominio.md): Approval aponta para uma **Artifact Revision** específica e nunca é herdada por uma Revision nova; toda Revision é imutável e tem origem única (Execution ou edição humana); Executions leem Revisions fixadas no início. A política de revalidação decide o que fazer com isso, não se a aprovação "migra".
