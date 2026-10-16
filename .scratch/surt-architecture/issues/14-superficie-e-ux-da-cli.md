# Superfície e UX da CLI

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 05, 08

## Question

Quais são os comandos, seus contratos e a experiência de acompanhamento?

Decidir: semântica de cada comando do brief (em especial `surt feature` × `surt plan` — o que cada um faz?); como se referencia uma feature/run (`20261015-...`, prefixo de UUID?); saída de `status`/`inspect` (humano × `--json`); `logs` ao vivo de várias execuções paralelas (tail de `stream.jsonl`? multiplexado?); progresso em foreground; modo não interativo (CI) e exit codes; mensagens de erro acionáveis; `surt init` (o que gera, `.gitignore`, agentes embutidos materializados ou não); confirmações destrutivas.

Saída: seção J.

## Context

De [Modelo de processo e controle de concorrência](05-modelo-de-processo-e-concorrencia.md), já decidido: `surt feature` dirige até o gate da spec; `surt approve` **só registra**; `surt plan` dirige o Task Agent até o gate das tasks (exige spec aprovada); `surt start` dirige até a entrega do Shipper (exige tasks aprovadas); reexecutar o mesmo comando retoma (sem `surt resume` no M1); `surt cancel` grava um pedido pelo SQLite; um comando que encontra o run com Driver vivo recusa com pid e instruções. Falta aqui: referência de feature/run, saídas de `status`/`inspect` (dono, vagas `4/4 in use`, `QUEUED`), exit codes, `logs`.
