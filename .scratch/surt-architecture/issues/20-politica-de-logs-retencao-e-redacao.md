# Política de logs, retenção e redação

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 07, 08

## Question

O que o Surt persiste de `stream.jsonl` e `stderr.log` por Execution, por quanto tempo e com qual política de redação de dados sensíveis?

Decidir: eventos originais da CLI versus eventos normalizados; captura padrão versus opt-in de logs completos; metadados mínimos que permanecem no SQLite mesmo sem logs; tratamento de caminhos, prompts, conteúdo de ferramentas e possíveis segredos sem prometer detecção perfeita; retenção por estado do run; o que o cleanup remove e quais evidências ainda permitem diagnóstico/recuperação. A política não pode transformar logs em fonte de verdade do fluxo nem depender de um timer de execução.

## Context

Graduado da névoa do mapa após [Contrato de agente](07-contrato-de-agente.md): o [contrato do runtime](../../../docs/architecture/F-agent-contract.md#runtime-success-and-failure) fixa interpretação dos eventos, aceitação da entrega e metadados disponíveis, mas deixa captura, retenção e redação como decisão própria. [Armazenamento de artefatos, congelamento e revalidação](08-armazenamento-congelamento-revalidacao.md) define o armazenamento com o qual esta política precisa coexistir. [Segurança do cleanup e do ciclo de vida](13-cleanup-e-ciclo-de-vida.md) aplica a política decidida aqui.

De [Armazenamento de artefatos, congelamento e revalidação](08-armazenamento-congelamento-revalidacao.md): cada Execution tem `.surt/runtime/executions/<id>/{inputs,delivery}/`, recriado por Attempt e removido no cleanup; decidir se logs/stream ficam nesse diretório ou à parte. Ver [H. Artifact Storage](../../../docs/architecture/H-artifact-storage.md).
