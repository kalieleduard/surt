# Modelo de integração Git

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Assignee: Kaliel
Blocked by: 01, 04

## Question

Como o código flui das branches de Task até a branch entregue pelo Shipper, garantindo que nada não-revisado entre?

Decidir: nome e base da branch de integração por feature; de qual commit cada branch de Task parte (integração no momento do start?); quem integra uma Task aprovada (operação nativa determinística × agente) e quando; o que acontece em conflito (nova iteração do Builder rebaseada? re-review obrigatório?); como o review fica vinculado ao SHA exato (e o que invalida a aprovação); quem roda build/lint/test (comandos declarados em config e executados como step nativo? resultados viram artefato para o Reviewer?); o papel exato do Shipper (agente × nativo × híbrido) e o que ele entrega (branch integrada final, sem merge na main); layout de `~/.surt/worktrees/<project>/<feature>/<task>/` e identificação de projeto.

## Context

De [Contrato de agente](07-contrato-de-agente.md): [F. Agent Contract](../../../docs/architecture/F-agent-contract.md) fixa schemas de `commit`, `feedback` e `delivery`; este ticket define prova Git/vínculo de SHA, comandos de checks e comportamento final do Shipper. Os `.md` por Task chegam pelo mecanismo genérico de item files; o mapeamento de chaves para paths físicos deve respeitar as três plataformas, sem usar uma key não validada diretamente como nome de arquivo.

Surgiu de [Mecânica de worktree/merge no Git e estados parciais](04-git-worktree-merge-estados-parciais.md) — decidir aqui: integração via `merge-tree` + `commit-tree` + `update-ref` (sem worktree) × merge dentro de worktree; versão mínima do Git exigida (≥2.38/2.40/2.43); CLI `git` × libgit2/gix (a pesquisa aponta CLI); nunca mover via `update-ref` uma branch checada em outra worktree; vínculo do review ao SHA e política de `core.autocrlf` no Windows; configs que o Surt impõe ou checa (`core.longpaths`, `core.lockfilePid`).

De [Modelo de processo e controle de concorrência](05-modelo-de-processo-e-concorrencia.md): as Native Operations do Surt que mexem no estado compartilhado do repo (worktree add/remove, criação de branch, `update-ref`, `merge-tree`/`commit-tree`) rodam serializadas sob `.surt/runtime/locks/git.lock`, com retry com backoff para locks do Git criados por terceiros; commits do Builder na própria worktree ficam fora desse lock.

De [Schema TOML de workflow](06-schema-toml-de-workflow.md): as Operations são referenciadas no TOML só por id (`run = "git.integrate"`); este ticket decide o **registro de operações** (ids, argumentos, Outcomes declarados). Rascunho em uso na seção E: `git.create_integration_branch` (feature, `prepare`), `git.create_task_worktree` (task-cycle, `setup`), `checks.run` → `passed`/`failed` (placeholder do step `check`, com artefato `check-report` para o Reviewer) e `git.integrate` → `merged`/`conflict`. No task-cycle, `integrate` roda dentro do Loop após `review = approved`, e `conflict` dispara nova Iteration: decidir o que o Builder recebe sobre o conflito. O Shipper lê só o `manifest` + a branch de integração (o pai não lê artefatos dos filhos). Também aqui: restrições das `key` da Item List para virarem nomes de branch/diretório (nomes reservados do Windows, caracteres, colisão case-insensitive).

De [Armazenamento de artefatos, congelamento e revalidação](08-armazenamento-congelamento-revalidacao.md): arquivos de item (`tasks/<key>.md`) espelham os paths relativos da Revision na Working Copy; o encoding seguro key→path (Windows) é decidido aqui. O Shipper **não** commita a spec na branch; se o texto de entrega resume a spec é decisão deste ticket. Builders leem inputs de `.surt/runtime/executions/<id>/inputs/`, nunca de dentro da worktree. Ver [H. Artifact Storage](../../../docs/architecture/H-artifact-storage.md).

## Answer

Resolvido por troca humana em duas rodadas (mais um ajuste no "allow merge"). Seção completa em [A. Git Integration](../../../docs/architecture/A-git-integration.md). O documento é parte da seção A (arquitetura), não da B: a letra B do brief é o workspace Rust (ticket 15), e as menções antigas a "section B" em E/F/H foram corrigidas. Trade-offs em [ADR 0006 — Merging into the Base is opt-in and fast-forward only](../../../docs/adr/0006-merge-into-base-is-opt-in-fast-forward.md) e [ADR 0007 — Tasks are integrated as merge commits of the reviewed SHA](../../../docs/adr/0007-integrate-reviewed-sha-as-merge-commit.md). O glossário ganhou **Base**, **Integration Branch**, **Task Branch**, **Conflict**, **Check** e **Initial Artifact**, e o Shipper foi redefinido como só de leitura.

Decisões finais:

- **Integração:** Operation nativa `git.integrate`, sem worktree: `merge-tree --write-tree` → `commit-tree` (datas fixas = início da Execution, SHA determinístico) → `update-ref --stdin` com CAS sob `git.lock`. É sempre um merge commit com pais `[ponta, SHA revisado]`, nunca squash/rebase/ff. Trailers `Surt-Run`/`Surt-Task`/`Surt-Reviewed-Commit`; identidade do usuário. Git ≥ 2.38, só CLI.
- **Nomes:** integração `surt/<subject>`, Task `surt/task/<subject>/<key>` (evita o conflito arquivo×diretório de refs); runs N ≥ 2 do mesmo Subject usam `<subject>.N`.
- **Base:** SHA de `HEAD` (ou de `--base <ref>`) em `surt feature`, gravado como Initial Artifact `base` `{ref, sha}` (o engine generaliza o `request` para "initial artifacts" do comando criador). O `prepare` cria a integração exatamente ali; avisa se houver mudanças não commitadas.
- **Start das Tasks:** ponta da integração no `setup` da Task, gravada como `start`. A leitura do start e todo o `integrate` ficam serializados por root run (mutex no Driver).
- **Worktrees:** `~/.surt/worktrees/<project-id>/<subject>[.N]/<key>/`, caminho gravado no SQLite. Project id = `<dir-slug≤24>-<6hex>` gerado ao criar o `surt.db`.
- **Keys/paths:** regras do engine para toda Item List: key `^[A-Za-z0-9][A-Za-z0-9_-]{0,31}$`, única case-insensitive, sem nomes reservados do Windows; componentes de path `[A-Za-z0-9._-]+` com regras equivalentes; uso literal, sem encoding. O schema do manifest foi apertado.
- **Checks:** `[[checks]]` argv (sem shell) com `timeout` opcional em `.surt/config.toml`, congelados no root run (nunca lidos da worktree). `checks.run` roda todos, em ordem, no SHA do Builder → `check-report` (status, exit, duração, últimas 200 linhas). `failed` pula review e integrate (`when` no `review`), e o Builder recebe `check-report?`.
- **Prova do commit:** existe; é `HEAD` e ponta da Task Branch; descende do start (e da ponta, se houve conflito) com commit novo; worktree limpa e sem operação em andamento. Falha = entrega inválida.
- **Vínculo:** `check-report.commit` = `feedback.reviewed_commit` = `commit.sha`; o `integrate` mergeia o SHA aprovado, não a branch. Só SHA novo invalida a aprovação.
- **Merge limpo × quebrado:** se a ponta andou desde o start, o merge é re-checado na worktree da Task (`checkout --detach M`) antes do `update-ref`. Invariante: toda ponta da Integration Branch passou nos Checks.
- **Conflito:** `git.integrate` → `conflict` com artifact `conflict` (`kind = merge|checks`, ponta, SHA revisado, paths ou checks). O Builder faz merge (não rebase) da ponta e passa de novo por check e review completos. O Context Assembler rotula cada Input com Iteration/step de origem e diz por que a Iteration começou (F atualizada).
- **Posse da Integration Branch:** recusa se estiver checada em alguma worktree; para (sem adotar) se a ponta divergir da gravada; sempre mostra o comando de correção.
- **Shipper:** novo step `snapshot` (`git.snapshot_integration` → artifact `integration` {branch, base, tip, tasks}). O Shipper é só leitura, lê `manifest` + `integration` e entrega `delivery` = `{title, summary}`; branch/SHA vêm do `integration`. Nada é pushado; a spec não é commitada.
- **Merge na Base (opt-in):** step `merge` (`git.merge_into_base`) depois do `ship`, controlado por `[git] merge_into_base = "never" | "ff-only"` (default `never`, congelado). Só fast-forward: pula se a Base andou, se não é branch ou se a worktree onde ela está checada não está limpa; em worktree limpa usa `merge --ff-only` lá dentro. Outcomes `merged` · `skipped`. A restrição do brief foi relaxada para "salvo opt-in".
- **Config Git:** `--no-optional-locks` nas leituras; `-c core.longpaths=true` (Windows) e `-c core.lockfilePid=true` (Git ≥ 2.56) nos comandos do Surt; `core.longpaths` injetado no agente via `GIT_CONFIG_*`; nunca escreve `.git/config`; não impõe `autocrlf`. Preflight: versão, repo não-bare, identidade.

Validação documental: todos os blocos TOML de A/E/F fazem parse (`tomllib`); os schemas JSON novos (`base`, `check-report`, `conflict`, `integration`, `delivery`) fazem parse. Notas de contexto acrescentadas a [Máquina de estados do loop Builder ↔ Reviewer](10-loop-builder-reviewer.md), [Schema SQLite](11-schema-sqlite.md), [Estratégia de recuperação e reconciliação](12-recuperacao-e-reconciliacao.md), [Segurança do cleanup e do ciclo de vida](13-cleanup-e-ciclo-de-vida.md), [Superfície e UX da CLI](14-superficie-e-ux-da-cli.md), [Estrutura do workspace Rust e traits centrais](15-workspace-rust-e-traits.md) e [Estratégia de testes](16-estrategia-de-testes.md). Nenhum ticket novo: o que sobrou cabe nesses tickets.
