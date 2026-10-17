# Roadmap + proposta do Milestone 1

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 13, 15, 16

## Question

Qual é o menor Milestone 1 que já entrega uma Feature de ponta a ponta, e o que vem depois?

Decidir:
- **M1:** objetivo, módulos Rust, arquivos, testes e critério de pronto. Critério natural: o E2E do brief passa com o Fake Runtime e uma Feature pequena passa com o Claude Code de verdade.
- **Corte de comandos:** dos 14 de [J](../../../docs/architecture/J-cli.md), quais entram no M1. Candidatos a sair: `logs`, `inspect <execution-id>`, `--json`, rodapé ao vivo, `--new-run`, `stop` — sem quebrar exit codes e mensagens.
- **Corte de mecanismos já decididos:** o que foi desenhado nos tickets resolvidos mas pode ficar para depois sem mudar o schema (ex.: pausa por Transient Failure, `merge_into_base`, Salvage Refs, `restore`). Para cada um, confirmar que adiar não exige migration destrutiva.
- **Milestones seguintes:** ordem e objetivo em uma linha cada.

Candidatos de corte vindos de [I. Cleanup](../../../docs/architecture/I-cleanup.md): `on_success = "keep"`, salvage antes de remover worktree no `cancel`, `surt delete` (e a checagem de `--new-run` concorrente). `cleaned_at` e `.surt/logs/` já estão no schema/layout; adiar não exige migration.

Saída: seções K e L (ADRs pendentes consolidados). Fecha o mapa.
