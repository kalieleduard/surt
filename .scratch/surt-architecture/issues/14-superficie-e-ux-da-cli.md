# Superfície e UX da CLI

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 05, 08

## Question

Quais são os comandos, seus contratos e a experiência de acompanhamento?

Decidir: semântica de cada comando do brief (em especial `surt feature` × `surt plan` — o que cada um faz?); como se referencia uma feature/run (`20261015-...`, prefixo de UUID?); saída de `status`/`inspect` (humano × `--json`); `logs` ao vivo de várias execuções paralelas (tail de `stream.jsonl`? multiplexado?); progresso em foreground; modo não interativo (CI) e exit codes; mensagens de erro acionáveis; `surt init` (o que gera, `.gitignore`, agentes embutidos materializados ou não); confirmações destrutivas.

Saída: seção J.
