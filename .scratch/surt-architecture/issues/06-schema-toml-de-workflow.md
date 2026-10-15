# Schema TOML de workflow

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 01

## Question

Qual é o menor schema TOML que expressa o workflow de feature e o task-cycle sem virar uma linguagem?

Decidir: tipos de step (agent, human_approval, foreach/subworkflow, native) e seus campos; como se declaram **execution dependencies** (`needs`) × **data dependencies** (`inputs` referenciando artefatos de outros steps); como o foreach expande dinamicamente sobre Tasks que só existem depois do Task Agent rodar, respeitando dependências entre Tasks declaradas por elas mesmas; como o subworkflow recebe parâmetros e devolve outputs; condições/guards (ex.: decisão do reviewer) sem expressões arbitrárias; validação estática (ciclos, inputs inexistentes, tipos de step); versionamento do workflow e snapshot no run; workflows embutidos × overrides em `.surt/workflows/`; políticas de cleanup por workflow.

Saída: seção E com `feature.toml` e `task-cycle.toml` completos que fazem parse.
