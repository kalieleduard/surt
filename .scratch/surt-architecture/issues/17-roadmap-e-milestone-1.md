# Roadmap + proposta do Milestone 1

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 10, 13, 14, 15, 16

## Question

Qual é a sequência de milestones que respeita as dependências técnicas, e o que exatamente é o Milestone 1?

Decidir: ordem final dos milestones (ajustando a lista do brief), e para cada um: objetivo, escopo, componentes, entregáveis, testes, critérios de aceitação; para o M1: arquivos, módulos Rust, testes e critérios de conclusão. Consolidar ADRs pendentes (seção L) com trade-offs, riscos e funcionalidades adiadas.

Saída: seções K e L. Fecha o mapa.

De [Superfície e UX da CLI](14-superficie-e-ux-da-cli.md) ([J. CLI Surface](../../../docs/architecture/J-cli.md)): a superfície completa do M1 tem 14 comandos; decidir aqui quais entram no M1 de fato e quais (`logs`, `inspect <execution-id>`, `--json`, rodapé ao vivo, `--new-run`) podem ficar para um milestone seguinte sem quebrar o contrato de exit codes e mensagens.
