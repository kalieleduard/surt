# Superfície e UX da CLI

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Blocked by: 05, 08
Assignee: Kaliel

## Question

Quais são os comandos, seus contratos e a experiência de acompanhamento?

Decidir: semântica de cada comando do brief (em especial `surt feature` × `surt plan` — o que cada um faz?); como se referencia uma feature/run (`20261015-...`, prefixo de UUID?); saída de `status`/`inspect` (humano × `--json`); `logs` ao vivo de várias execuções paralelas (tail de `stream.jsonl`? multiplexado?); progresso em foreground; modo não interativo (CI) e exit codes; mensagens de erro acionáveis; `surt init` (o que gera, `.gitignore`, agentes embutidos materializados ou não); confirmações destrutivas.

Saída: seção J.

## Context

De [Contrato de agente](07-contrato-de-agente.md): a aprovação do manifesto cobre seu índice e todos os `.md` por Task, e editar qualquer documento exige nova Revision e reaprovação. A UX precisa permitir inspecionar esse conjunto; não apresentar aprovação de um índice isolado como aprovação das instruções. Contrato e configuração de agentes: [F. Agent Contract](../../../docs/architecture/F-agent-contract.md).

De [Modelo de processo e controle de concorrência](05-modelo-de-processo-e-concorrencia.md), já decidido: `surt feature` dirige até o gate da spec; `surt approve` **só registra**; `surt plan` dirige o Task Agent até o gate das tasks (exige spec aprovada); `surt start` dirige até a entrega do Shipper (exige tasks aprovadas); reexecutar o mesmo comando retoma (sem `surt resume` no M1); `surt cancel` grava um pedido pelo SQLite; um comando que encontra o run com Driver vivo recusa com pid e instruções. Falta aqui: referência de feature/run, saídas de `status`/`inspect` (dono, vagas `4/4 in use`, `QUEUED`), exit codes, `logs`.

De [Schema TOML de workflow](06-schema-toml-de-workflow.md): rejeitar num Approval Gate é um comando próprio, **`surt reject <feature> -m "<motivo>"`** (não `surt approve --reject`). A rejeição dispara uma nova Iteration do Loop do gate (o agente refaz lendo a Revision rejeitada + o motivo). Decidir aqui: se `reject`, como `approve`, só registra, e qual comando dirige a regeneração depois (reexecutar `surt feature <feature>` / `surt plan <feature>`?).

De [Armazenamento de artefatos, congelamento e revalidação](08-armazenamento-congelamento-revalidacao.md): comandos novos/flags a especificar: `surt show <subject> <artifact> [--rev N]`, `surt restore <subject> <artifact>`, `surt approve --rev N` (trava), `surt feature --name <slug>`; toda ação da feature varre a Working Copy primeiro; mensagens de recusa esboçadas (spec não aprovada, edição de Item List inválida, Revision fixada). `surt init` gera `.surt/.gitignore` (só `config.toml` e `agents/` versionados). Ver [H. Artifact Storage](../../../docs/architecture/H-artifact-storage.md).

De [Modelo de integração Git](09-modelo-de-integracao-git.md) ([A. Git Integration](../../../docs/architecture/A-git-integration.md)): flags a especificar: `surt feature --base <ref>` e um override por run de `[git] merge_into_base` (ex.: `--merge`), aplicado ao congelar a config. O `status` mostra a Base (`base main @ a11c9d0`), a Integration Branch e a ponta, o modo de merge efetivo, e por Task o SHA merged ou o motivo do conflito (`merge: src/auth.rs` / `checks: test`). Mensagens de recusa já esboçadas na seção: Git velho, sem identidade, branch checada em worktree, ponta movida por fora, `not merged: main moved since base`. `surt show <subject> delivery` mostra o texto do Shipper.

De [Máquina de estados do loop Builder ↔ Reviewer](10-loop-builder-reviewer.md) ([G. State Machines](../../../docs/architecture/G-state-machines.md)): `status` mostra o estado derivado do run (`RUNNING`, `AWAITING APPROVAL`, `BLOCKED`, `INTERRUPTED`) e por Task o passo bloqueado com o último erro e o comando para retomar (mock na seção G). O driving command sai com erro quando para em `blocked` (definir os exit codes aqui), e ao retomar imprime `↻ unblocking … (3 more attempts)`. Consolidar aqui a posição de `max_attempts` (3) e `max_iterations` (5) no `.surt/config.toml`. Não há cancelamento por Task.

De [Schema SQLite](11-schema-sqlite.md) ([D. Database Schema](../../../docs/architecture/D-database.md)): ids são UUIDv7 em TEXT (iguais nos diretórios `runtime/executions/<id>/` e nos locks); Revisions aparecem como `rev N` (`artifact_revision.number`); `actor` de Approvals, edições e cancel = `git user.email` (fallback: usuário do sistema), útil para `approved by … at …`. A linha do tempo de `surt inspect` sai de `event` (kinds iniciais listados na seção D). O motivo de um bloqueio no `status` vem de `step_run.blocked_reason` + `error_message` da última Execution (query de exemplo na seção D).

De [Estratégia de recuperação e reconciliação](12-recuperacao-e-reconciliacao.md) ([I. Recovery and Reconciliation](../../../docs/architecture/I-recovery.md)): fixar texto e exit code de: recusa na partida com a Integration Branch movida ou checada em outra worktree (nenhum estado muda; `status` segue `INTERRUPTED`); linhas de recuperação (`⚠ previous driver … is gone — recovering`, órfão parado, Execution descartada, integração confirmada); aviso de salvage (só quando a sobra é inesperada); pausa por Transient Failure (`⏸ agent launches paused until 15:00`); bloqueio `merge_into_base_interrupted` com as duas instruções; `status` mostrando `⟳ integrating … (pending)` e órfão vivo sem Driver; `cancel` sem Driver imprimindo o que a recuperação encontrou.

## Answer

Resolvido por troca humana em três rodadas (18 perguntas, todas aceitas com a recomendação). Seção completa em [J. CLI Surface](../../../docs/architecture/J-cli.md). Seções A, D, G, H, I e E atualizadas para apontar para ela; `CONTEXT.md` (**Interruption**) passa a citar o pedido de parada de outro terminal. Sem ADR novo.

Decisões finais:

- **Superfície do M1:** `init`, `feature`, `plan`, `start`, `approve`, `reject`, `stop`, `cancel`, `status`, `inspect`, `logs`, `show`, `restore`, `delete`. O `resume` do brief é reexecutar o comando da fase; o `stop` do brief vira **`surt stop`**: Interruption remota pelo SQLite (nova coluna `stop_requested_at`/`_by` em `workflow_run`, kind `run.stop_requested`). O pedido vale só para o Driver vivo ao ser gravado e é limpo por quem pega o lock. `surt delete` é só nome reservado.
- **Referência:** nome completo, slug sem data se único, ou prefixo único; ambiguidade recusa listando; nunca "feature atual" implícita. Escrita age no run ativo; leitura usa ativo ou último, com `--run N`. Task pela key; Execution por id/prefixo ≥ 8. O Surt sempre imprime nomes completos.
- **`surt feature`:** argumento é referência só se for exatamente o nome completo existente. Descrição cujo slug bate com feature de run ativo recusa (mesmo depois da meia-noite) em vez de criar `-2`; `--new` força. `surt feature <nome>` retoma a fase da spec (regenera após reject; "já aprovada" sai 0). Novo run do mesmo Subject só com `--new-run` (request mais recente, Base = `HEAD` ou `--base`, recongela tudo). `--merge`/`--no-merge` só em `surt feature`.
- **`approve`/`reject` só registram** e terminam com o próximo comando (`next:`/`regenerate:`). A tabela gate → comando fica na camada de CLI do workflow `feature`, não no schema TOML nem no engine.
- **Leitura nunca escreve:** `status`/`inspect`/`show`/`logs` não gravam Revision, não recriam arquivo, não pegam lock — mostram `✎ … edited`. Corrige a lista "When" de [H. Edit detection](../../../docs/architecture/H-artifact-storage.md#edit-detection), que incluía os comandos de leitura.
- **Formatos:** `status` em lista (ativos; `--all`) e detalhe (Base, branch, modo de merge, gate mostrando o conjunto inteiro aprovado); `inspect` em três níveis (run, task, Execution); `show` com índice, `--file`, `--task --iter`; `logs` renderizado/multiplexado com `-f`/`--raw`; progresso append-only + rodapé ao vivo no TTY, linha de vida a cada 5 min sem TTY.
- **Canais:** progresso no stderr, dados no stdout; `--json` (`"schema": 1`) só em `status`/`inspect`/`show`; sem TTY nunca pergunta; `NO_COLOR`; fallback ASCII dos símbolos; `SURT_LOG` para diagnóstico.
- **Exit codes:** `0` ok · `1` interno · `2` uso · `3` recusado sem mudança · `4` blocked · `5` cancelado durante · `130` interrompido (Ctrl-C ou `surt stop`). Iguais nos três SOs.
- **Confirmações:** `cancel` e `restore` (e `delete`) confirmam mostrando o impacto; `--yes`; sem TTY e sem `--yes` = `3`.
- **Mensagens de erro:** `✖ o quê` + contexto ≤ 2 linhas + estado só quando não óbvio + 1–3 saídas com labels fixos (`fix:`, `next:`, `resume:`, `regenerate:`, `undo:`, `inspect:`, `or:`).
- **`surt init`:** explícito (os outros recusam fora de projeto), na raiz da worktree principal, com preflight de Git, idempotente, sem materializar agentes embutidos. Recusa dentro de worktree do Surt.
- **Config consolidada:** `.surt/config.toml` com `max_concurrent_agents`, `max_attempts`, `max_iterations` no topo, `[git] merge_into_base`, `[[checks]]`; global `~/.surt/config.toml` (`%USERPROFILE%` no Windows) com `stop_grace_period`. Chave desconhecida é erro. J é a referência única.

Mudança no schema (migration 0001 ainda não lançada, editada): `workflow_run.stop_requested_at`/`stop_requested_by` com CHECK de par e de "só root". Validação: migration roda em `sqlite3` 3.51 com `-bail`; o pedido grava e limpa, e meio-par é recusado pelo CHECK.

Notas de contexto acrescentadas a [Segurança do cleanup e do ciclo de vida](13-cleanup-e-ciclo-de-vida.md), [Estrutura do workspace Rust e traits centrais](15-workspace-rust-e-traits.md), [Estratégia de testes](16-estrategia-de-testes.md), [Roadmap e Milestone 1](17-roadmap-e-milestone-1.md) e [Política de logs, retenção e redação](13-cleanup-e-ciclo-de-vida.md) (depois absorvido pelo 13). Nenhum ticket novo; a névoa "conteúdo do config" foi consolidada e saiu do mapa.
