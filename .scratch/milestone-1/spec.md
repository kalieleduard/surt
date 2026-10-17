# Spec: Milestone 1 — uma Feature ponta a ponta que sobrevive a `kill -9`

Labels: ready-for-agent
Type: spec
Status: open
Map: [Surt — Arquitetura e plano de implementação](../surt-architecture/map.md)
Plano: [K. Roadmap and Milestone 1](../../docs/architecture/K-roadmap.md)

## Problem Statement

O desenvolvedor quer entregar uma Feature pequena com agentes de IA sem ficar de babá de uma sessão de chat: escrever o pedido, revisar a spec, revisar o plano de Tasks e receber uma branch pronta, com cada Task construída, revisada e integrada. Hoje não existe nada que faça isso de forma local, determinística e retomável. O repositório só tem um crate `ferris-says` descartável. Sessões manuais do Claude Code perdem o fio quando o terminal fecha. Ninguém garante que um commit não revisado entrou na branch. Um Ctrl-C, um `kill -9` ou um reboot no meio do trabalho deixam worktrees, branches e processos num estado que o humano precisa desembaraçar sozinho.

O plano A–L em `docs/architecture/` resolveu todas as decisões da v1. Falta o código. Este spec descreve o **Milestone 1**: o menor subconjunto da v1 que entrega uma Feature de ponta a ponta com o Claude Code de verdade e sobrevive a um `kill -9` em qualquer ponto.

## Solution

Um binário `surt` em Rust, local-first, com SQLite como fonte de verdade, que dirige o workflow embutido `feature` (com um `task-cycle` por Task) em três fases em foreground, cada uma parando onde precisa de um humano:

```
$ surt init
$ surt feature "add login"             # Spec Agent → para no gate da spec
$ $EDITOR .surt/features/20261017-add-login/spec.md     # opcional
$ surt approve add-login
$ surt plan add-login                  # Task Agent → manifest + tasks/*.md → gate
$ surt approve add-login
$ surt start add-login                 # builders, checks, review, integrate, shipper, cleanup
✔ task A merged (3c1aa20)
↻ task B build attempt 2
✔ task B merged (9e04b7d)
✔ shipper    delivered — surt/20261017-add-login @ 4e1b2c0
✔ run        succeeded
↻ cleanup    2 worktrees · 2 task branches · working copy
```

O resultado é uma Integration Branch `surt/<feature>` em que cada Task entrou como merge commit do SHA que passou nos Checks e foi aprovado pelo Reviewer, mais o texto de entrega do Shipper. Nada é empurrado e a branch principal não é tocada. Qualquer interrupção, de Ctrl-C a queda de energia, é retomada rodando de novo o comando da fase: a Recovery acontece dentro dele, sem modo especial.

O motor (`surt-engine`) é genérico e não conhece Spec, Task, Builder, Reviewer nem Shipper. O crate `surt` traz a CLI e o workflow embutido.

## User Stories

### Projeto e instalação

1. As a developer, I want `surt init` to create `.surt/` at the toplevel of my repository's main worktree, so that every later command finds the project from any subdirectory.
2. As a developer, I want `surt init` to check Git ≥ 2.38 and a configured user identity, and refuse with a fix if either is missing, so that I learn about prerequisites before the first agent spends tokens.
3. As a developer, I want `surt init` to be idempotent and never overwrite `config.toml` or `agents/`, so that rerunning it is safe.
4. As a developer, I want `surt init` to write a `.surt/.gitignore` that versions only `config.toml` and `agents/`, so that the database, logs, Working Copies and runtime files never pollute my Git history.
5. As a developer, I want `surt init` to generate a commented `config.toml` with the defaults (`max_concurrent_agents = 3`, `max_attempts = 3`, `max_iterations = 5`, `merge_into_base = "never"`, an example `[[checks]]`), so that I can see what I can tune.
6. As a developer, I want every other command to refuse with exit `3` outside an initialized project and point me at `surt init`, so that no command silently creates state somewhere unexpected.
7. As a developer, I want a command run inside a Surt-managed worktree under `~/.surt/worktrees/` to refuse, so that I don't accidentally drive a Feature from a Builder's scratch space.
8. As a developer, I want unknown keys in `.surt/config.toml` to be errors, so that a typo never silently falls back to a default.
9. As a developer, I want Surt to work the same on macOS, Linux and Windows, including locks, process trees, paths and exit codes, so that my team can use it on any machine.

### Criar e especificar uma Feature

10. As a developer, I want `surt feature "<description>"` to create a Feature named `YYYYMMDD-<slug>` from my description, so that Features sort by date and are easy to refer to.
11. As a developer, I want `--name <slug>` to override the derived slug, so that I can pick a short name for a long description.
12. As a developer, I want a Feature name that collides with an existing one to get an automatic `-N` suffix, so that I never have to invent a unique name by hand.
13. As a developer, I want Feature names that are reserved on Windows (`con`, `nul`, …) or not portable to be rejected or adjusted, so that a Feature created on macOS also works on Windows.
14. As a developer, I want `--base <ref>` to fix the commit my Feature starts from, with `HEAD` as the default, so that the Integration Branch is created exactly where I chose.
15. As a developer, I want the run to freeze the workflow definitions, the agents and the project configuration when it is created, so that editing `config.toml` later never changes a run already in progress.
16. As a developer, I want `surt feature "<description>"` to drive the Spec Agent and stop at the spec Approval Gate with exit `0`, so that I review the spec before anything else happens.
17. As a developer, I want rerunning `surt feature "add login"` while that slug has an active run to refuse and tell me how to resume or create anew, so that pressing ↑ Enter after a Ctrl-C never creates a duplicate Feature.
18. As a developer, I want `--new` to skip that guard, so that I can deliberately start a second Feature with the same description.
19. As a developer, I want `surt feature <full-name>` to resume the spec phase of the active run, regenerating after a rejection, so that resuming is "run the phase command again".
20. As a developer, I want `surt feature <full-name>` on an already approved spec to say so, point at `surt plan` and exit `0` without changing anything, so that rerunning it is harmless.
21. As a developer, I want `surt feature "<description>"` after a cancel to create a `-2` Feature, so that I can start over without `--new-run`, which Milestone 1 doesn't ship.

### Revisar, editar, aprovar e rejeitar

22. As a developer, I want the spec to appear as `.surt/features/<feature>/spec.md`, so that I can read and edit it in my editor.
23. As a developer, I want my edit to the spec to become a new human-origin Revision when I run `approve`, `reject` or a driving command, so that my change is versioned and the Approval targets exactly what I saw.
24. As a developer, I want `surt approve <feature>` to record an Approval on the waiting gate's latest Revision, print what it approved with its SHA-256 and author, and print the next command, so that I always know how to continue.
25. As a developer, I want `surt approve`/`reject` to never drive anything, so that recording a decision is instant and never launches an agent by surprise.
26. As a developer, I want `--rev N` to refuse if the latest Revision is not `N`, so that I never approve a Revision I didn't read.
27. As a developer, I want `surt reject <feature> -m "<reason>"` to record the rejection with my reason and point at the command that regenerates, so that the agent redoes the work reading my reason.
28. As a developer, I want a rejected spec to start a new Iteration of the spec loop whose Spec Agent reads the rejected spec and my reason, so that the next Revision addresses my feedback.
29. As a developer, I want loops that contain an Approval Gate to have no Iteration limit, so that I can reject as many times as I need.
30. As a developer, I want a deleted Working Copy file to be recreated from the latest Revision by the next command that acts, so that deleting a file is how I undo an unrecorded edit while `surt restore` isn't available.
31. As a developer, I want an edit to an approved but not yet consumed Revision to be recorded as an unapproved Revision, with every driving command refusing until I approve it, so that unapproved content never flows downstream.
32. As a developer, I want an edit to a Pinned Revision to be refused rather than recorded, with the Driver letting running work finish, launching nothing new and exiting `3`, so that implementation never runs against content different from what Builders already read.
33. As a developer, I want those refusals to say `fix: rm <file>, then rerun`, so that I have a way out without `surt restore`.
34. As a developer, I want an agent Revision that arrives while I have an unrecorded edit to first save my edit as its own Revision and warn me, so that nothing I wrote is lost.

### Planejar Tasks

35. As a developer, I want `surt plan <feature>` to refuse unless the spec is approved, so that the Task Agent never plans from an unapproved spec.
36. As a developer, I want `surt plan` to drive the Task Agent to the manifest Approval Gate, so that I review the Task breakdown before any code is written.
37. As a developer, I want the manifest and one Markdown file per Task to appear in `.surt/features/<feature>/manifest.json` and `tasks/<key>.md`, so that I can review and edit the Builder's instructions directly.
38. As a developer, I want approving the manifest to cover the JSON and every Task file together as one Revision, so that editing any `tasks/*.md` needs reapproval.
39. As a developer, I want an edit to the manifest set to be validated with the same rules as an agent delivery (schema, unique keys, valid `needs`, no cycles, every referenced file present, no unreferenced file, safe paths), and an invalid edit to create no Revision and refuse, so that a broken manifest never reaches implementation.
40. As a developer, I want rejecting the manifest to regenerate it with my reason, so that I can push back on Task size or ordering.

### Implementar, revisar e integrar

41. As a developer, I want `surt start <feature>` to refuse unless the manifest is approved, so that no Builder runs before human approval.
42. As a developer, I want `surt start` to create the Integration Branch `surt/<feature>` at the Base, so that all reviewed work collects in one Surt-owned branch.
43. As a developer, I want each Task to run in its own worktree on its own Task Branch `surt/task/<feature>/<key>`, starting from the Integration Branch tip when the Task begins, so that Tasks never step on each other or on my working tree.
44. As a developer, I want Tasks to start in the order their `needs` allow, running independent ones in parallel up to `max_concurrent_agents`, so that a Feature finishes as fast as its dependencies permit.
45. As a developer, I want every Builder to be a fresh Claude session with only its fixed Inputs (its item, its `task.md`, the spec and any `feedback`/`check-report`/`conflict`), so that no stale context leaks between Attempts or Iterations.
46. As a developer, I want a Builder's delivery to be accepted only if its commit exists, is the worktree's `HEAD` and the Task Branch tip, descends from the Task's start (and from the conflict's integration tip after a conflict), is new, and leaves a clean worktree, so that a SHA-shaped string alone never counts as work.
47. As a developer, I want a Builder that finds nothing to change to make an empty commit explaining why, so that every integrated Task still has a reviewed SHA.
48. As a developer, I want my frozen `[[checks]]` to run as argv, without a shell, in the Task worktree on the Builder's commit, all of them even after one fails, so that the Builder sees every failure at once.
49. As a developer, I want a failed Check to skip review and start a new Iteration whose Builder reads the `check-report`, so that no Reviewer time is spent on a commit that doesn't pass.
50. As a developer, I want the Reviewer to judge the whole commit and its verdict to be bound to that exact SHA, so that an approval can never be reused for different code.
51. As a developer, I want an `approved` review with a blocking finding to be rejected as an invalid delivery, so that the verdict is always consistent.
52. As a developer, I want `changes_requested` to start a new Iteration whose Builder reads the feedback, and the next Reviewer to verify its earlier blocking findings without moving the bar, so that review converges.
53. As a developer, I want an approved Task to be integrated as a merge commit of the reviewed SHA, computed without a worktree and with deterministic dates, so that every Task's reviewed commit is provably in the Integration Branch's history.
54. As a developer, I want integration to rerun the Checks on the merge result when the Integration Branch moved since the Task started, so that every tip of the Integration Branch passed the Checks.
55. As a developer, I want a merge conflict or a failed re-check to send the Task back to its Builder with a `conflict` artifact, so that the Builder merges the tip, resolves and goes through check and review again.
56. As a developer, I want integration to move the Integration Branch only from the tip Surt recorded, so that commits made outside Surt never enter silently.
57. As a developer, I want the Shipper to run only after every Task merged, read the manifest and the integration snapshot, and deliver a `title` and `summary` without touching Git, so that I get a ready-made pull request description for a complete Feature.
58. As a developer, I want `surt start` to end with exit `0`, the delivered branch and SHA, and the run `succeeded`, so that scripts and humans both know it's done.

### Limites e `blocked`

59. As a developer, I want a step whose Executions fail technically to retry at once, up to `max_attempts` failed Attempts, and then become `blocked`, so that a flaky agent gets another chance but a broken one doesn't burn tokens forever.
60. As a developer, I want a Task Cycle that runs `max_iterations` Iterations without merging to become `blocked`, so that an endless review loop stops for a human.
61. As a developer, I want failures that retrying cannot fix (auth, missing or mismatched model, missing executable, Integration Branch moved or checked out, foreign worktree path) to block on the first occurrence, so that no Attempts are wasted.
62. As a developer, I want a rate limit, overload or network failure to block the step at once without spending an Attempt, with exit `4` and the reset time when the stream reports one, so that I know when to come back.
63. As a developer, I want a blocked Task to hold back only the Tasks that depend on it while independent Tasks keep running and merging, so that one stuck Task doesn't stop all work.
64. As a developer, I want a run to never "fail", only to be blocked until I resume or cancel it, so that no work is ever thrown away by a limit.
65. As a developer, I want `surt start` to exit `4` when it stops on a blocked step, listing each blocked step with its last error, its log path and the command to resume, so that I can investigate without opening SQLite.
66. As a developer, I want rerunning the phase command to unblock every blocked step with a fresh budget while keeping the counters' history (`attempt 4`, `iter 6`), so that resuming is my explicit decision and history stays honest.

### Interrupção, cancelamento e recuperação

67. As a developer, I want Ctrl-C (or SIGHUP, or closing the console) to stop running agents gracefully, let a running Operation finish, and exit `130`, so that I can stop at any time and resume later.
68. As a developer, I want a second Ctrl-C to force-kill the agents' whole process trees, so that I'm never stuck waiting.
69. As a developer, I want interrupted Executions to never count toward `max_attempts`, so that pressing Ctrl-C five times never exhausts a step.
70. As a developer, I want a second driving command on a run that already has a live Driver to refuse with the Driver's pid, so that two processes never advance the same Feature.
71. As a developer, I want a `kill -9`, crash or reboot to release the run lock automatically, so that the next command can take over without manual cleanup.
72. As a developer, I want the next driving command to stop any agent left running by a dead Driver and mark it `interrupted`, so that no orphan keeps editing a worktree unattended.
73. As a developer, I want an Execution whose end was never recorded to be discarded even if it left a valid delivery, so that Surt never adopts work it didn't see finish.
74. As a developer, I want an integration that was interrupted between recording its Intention and confirming it to be confirmed if the ref moved, redone if it didn't, and blocked if someone else moved it, so that a crash never double-merges or loses a merge.
75. As a developer, I want every Attempt in a worktree to start from a clean worktree at the last accepted tip, with leftovers saved to a Salvage Ref and a warning naming it, so that half-done work is never continued but never lost either.
76. As a developer, I want a missing, half-created or orphaned Task worktree to be repaired automatically, and a path Surt didn't create to block instead of being deleted, so that Surt fixes what it owns and never touches what it doesn't.
77. As a developer, I want a driving command to refuse to start while the Integration Branch was moved or is checked out, with the exact `git` command to restore it, so that unreviewed commits never enter and I know how to fix it.
78. As a developer, I want a crash between publishing an agent Revision and writing its Working Copy file to never be mistaken for my edit, so that recovery doesn't invent human Revisions.
79. As a developer, I want `surt cancel <feature>` to confirm first, showing what will be stopped and what stays, so that I don't end a run by accident.
80. As a developer, I want `surt cancel` without a TTY and without `--yes` to refuse with exit `3`, so that scripts never hang on a prompt.
81. As a developer, I want `surt cancel` with no live Driver to stop orphans, settle pending Intentions so history says where the branch really is, and cancel the run, so that cancelling a crashed run is one command.
82. As a developer, I want `surt cancel` while another terminal drives the run to make that Driver stop and exit `5`, so that cancellation works across terminals through SQLite alone.

### Cleanup

83. As a developer, I want a `succeeded` run to remove the Working Copy, execution directories, Task worktrees, Task Branches still at their recorded tip and Salvage Refs, while keeping the Integration Branch, the history and the logs, so that my disk and ref list stay clean without losing anything accepted.
84. As a developer, I want a `cancelled` run to salvage and remove the worktrees but keep Task Branches and Salvage Refs, so that unintegrated work survives as cheap refs.
85. As a developer, I want a Working Copy file with an unrecorded edit, a Task Branch someone moved, or a foreign path to be kept with a warning instead of removed, so that cleanup never destroys my work.
86. As a developer, I want a cleanup that fails partway (a file held open on Windows) to stay pending and be finished by the next command that takes the run's lock, so that I never clean up by hand.

### Leitura

87. As a developer, I want `surt status` to list every Feature with an active run, its derived state (`RUNNING`, `AWAITING APPROVAL`, `BLOCKED`, `INTERRUPTED`), its Driver and its progress, and `--all` to add terminal runs, so that I see everything at a glance.
88. As a developer, I want `surt status <feature>` to show the Base, the Integration Branch, each gate's Revision and Approval, and each Task's state, Iteration, Attempt and merged SHA, so that I understand where the run is.
89. As a developer, I want `surt status` at an Approval Gate to list every file the Approval covers and any unrecorded edit, so that I know what I'm approving.
90. As a developer, I want `surt status` to warn when an agent is still running unattended with no Driver, and to show pending integrations and cleanup pending, so that I never mistake an orphan for a stopped run.
91. As a developer, I want `surt show <feature>` to list the artifacts and `surt show <feature> <artifact> [--rev N] [--task <key>]` to print the content byte for byte to stdout, so that I can read any Revision after cleanup and redirect it to a file.
92. As a developer, I want `show` to warn on stderr when the Working Copy holds an unrecorded edit, so that I know the printed content isn't what's on disk.
93. As a developer, I want `status` and `show` to never take a lock, recover, record an edit, recreate a file or write anything, so that they're safe in `watch`, scripts and read-only checkouts.
94. As a developer, I want a `<feature>` argument to accept the full name, a unique slug or a unique prefix, and an ambiguous one to refuse listing the candidates, so that I type little without ever acting on the wrong Feature.

### Saída e códigos

95. As a developer, I want progress, warnings and errors on stderr and only data on stdout, so that `surt show … > file` writes only the content.
96. As a developer, I want every error to follow one shape (`✖` line, at most two context lines, a state line only when work happened, one to three labelled copyable commands with full names), so that I always know what failed and what to type next.
97. As a developer, I want exit codes `0` done, `1` internal error, `2` invalid usage, `3` refused (nothing changed), `4` blocked, `5` cancelled while driving, `130` interrupted, identical on the three OSes, so that scripts can react without parsing text.
98. As a developer, I want an unexpected error to print `✖ internal error: …` and suggest `SURT_LOG=debug`, so that I can report or debug it.
99. As a developer, I want `NO_COLOR` to disable colour, so that logs and CI output stay clean.
100. As a developer, I want `SURT_LOG` to enable Surt's own diagnostic logs on stderr for any command, so that I can debug Surt itself without a special build.
101. As a developer, I want each agent's stream and stderr written to `.surt/logs/<feature>/<execution-id>/`, surviving cleanup, so that I can read what an agent did with ordinary tools while `surt logs` isn't available.

### Cortes visíveis do Milestone 1

102. As a developer, I want a cut command or flag (`stop`, `inspect`, `logs`, `restore`, `delete`, `--new-run`, `--json`, `--run`, `--iter`, `--file`, `--merge`/`--no-merge`) to not exist at all and exit `2`, so that nothing half-works.
103. As a developer, I want `merge_into_base = "ff-only"` to refuse with exit `3`, so that I learn it isn't supported yet instead of thinking my branch was merged.
104. As a developer, I want driving commands to refuse with exit `3` when `.surt/agents/` exists, so that my overrides are never silently ignored.

### Claude Code de verdade

105. As a developer, I want every agent to run on my existing Claude Code subscription login, without an API key, so that using Surt doesn't change how I'm billed.
106. As a developer, I want each Execution to be a fresh session (`--session-id` new, `--no-session-persistence`, never `--resume`/`--continue`), so that no Attempt inherits a previous conversation.
107. As a developer, I want the agent's environment built from an allowlist rather than inherited, so that variables from my shell or a parent Claude session (gateway URLs, model overrides, API keys) never change auth or model.
108. As a developer, I want agents to run with `dontAsk`, explicit tools per role, MCP denied, hooks, slash commands, memory and CLAUDE.md loading disabled, so that each role can do exactly its job and nothing prompts me mid-run.
109. As a developer, I want an Execution accepted only when the process exited 0, initialization matches the requested session, model, version, permission mode and tools, the result is a success with `is_error = false` and a structured output, the envelope is valid and every declared file passes validation, so that a success-shaped stream is never mistaken for a delivery.
110. As a developer, I want Claude Code older than 2.1.301 or missing to be a launch error that blocks at once with an actionable message, so that I know to upgrade.
111. As a developer, I want stopping an agent to stop its whole process tree (process group on Unix, Job Object on Windows) with a 10-second grace period before a forced kill, so that no subprocess outlives Surt.

### Mantenedor do Surt

112. As a Surt maintainer, I want CI to run `cargo fmt --check`, `cargo clippy --all-targets --features test-support -D warnings` and `cargo test --features test-support` on macOS, Linux and Windows with Git ≥ 2.38, so that every PR keeps the three platforms green.
113. As a Surt maintainer, I want a Fake Runtime that goes through the same delivery validation as Claude Code and can simulate valid deliveries, invalid JSON, missing files, model mismatch, crashes, limit failures and an Execution that waits until stopped, so that the whole flow is testable without spending tokens.
114. As a Surt maintainer, I want the Fake and fault points gated behind a `test-support` cargo feature that release builds never enable, so that test hooks can't reach users.
115. As a Surt maintainer, I want the brief's E2E to run through the binary in under 60 s per OS without real sleeps, so that the suite stays fast enough to run on every PR.
116. As a Surt maintainer, I want fault points at each of the eight transactional checkpoints and one real `kill` smoke, so that I can prove recovery from a crash between any Intention and its effect.
117. As a Surt maintainer, I want the engine crate to be unable to import the feature workflow, so that the compiler enforces that the core stays generic.
118. As a Surt maintainer, I want a written manual smoke checklist for the real Claude Code on macOS, so that "Milestone 1 done" is a repeatable check, not a feeling.

## Implementation Decisions

Toda decisão abaixo já está resolvida nos documentos do plano. Este spec só a consolida para o M1. Uma divergência entre este texto e `docs/architecture/` se resolve pelo documento do plano.

### Workspace e crates

- O crate `ferris-says` é substituído por um workspace com dois crates ([ADR 0011](../../docs/adr/0011-two-crates-keep-the-engine-generic.md)):
  - **`surt-engine`** (lib) é o motor genérico. Não pode depender de `surt`, e engine tests constroem workflows sintéticos.
  - **`surt`** (bin) traz clap, as precondições de fase de `plan`/`start`, a tabela gate → próximo comando, todo texto de mensagem, os exit codes e os assets embutidos (`feature`, `task-cycle` e os cinco Agents com TOML, instruções e schemas).
- Toolchain: `edition = "2024"`, `rust-version = "1.94"`, stable pinado em `rust-toolchain.toml`, Tokio multi-thread. As dependências permitidas são as da tabela de [B](../../docs/architecture/B-workspace.md#toolchain-and-dependencies), que é teto, não piso. Proibidos: `async-trait`, `git2`/`gix`, `chrono`/`time`, `indicatif`/`crossterm`, `include_dir`, `dirs`.
- A divisão de módulos de `surt-engine` é fixa (`project`, `workflow`, `drive`, `reconcile`, `artifacts`, `context`, `runtime`, `operations`, `git`, `cleanup`, `store`, `platform`). Nomes de arquivo dentro de cada módulo podem mudar. Todo `#[cfg]` de SO vive em `platform`.

### Interface do motor: a fachada `Project`

- A CLI vê um único tipo público, `Project`, e os tipos que ele devolve. O resto do motor é `pub(crate)`.
- Métodos no M1: `open`, `find_subject`, `create_run`, `drive`, `approve`, `reject`, `cancel`, `status`, `show`. `request_stop`, `restore`, `delete` e `inspect` ficam para o M2 e não precisam existir.
- **`drive` é a sequência inteira:** preflight → `try_lock` do root run → reconcile (passos 3–6 de [I](../../docs/architecture/I-recovery.md#what-a-driving-command-does-when-it-takes-the-lock)) → avançar até nada poder rodar sem humano → cleanup se terminal. É o único primitivo que dirige.
- Fases não existem no motor: `surt plan` confere "spec aprovada" num `StatusView` antes de chamar `drive`. Confirmações também não: `cancel` age quando chamado, e a CLI pergunta antes.
- A resolução de Subject (nome completo → slug único → prefixo único) mora no motor.
- **Resultados e erros:** `DriveOutcome` = `AtGate` (0) · `Succeeded` (0) · `Blocked` (4) · `Cancelled` (5) · `Interrupted` (130) · `Halted(Refusal)` (3, depois que o trabalho em curso terminou). `Error` = `Refused(Refusal)` (3, nada mudou) · `Internal` (1). clap mantém o 2. `Refusal` carrega valores observados e nenhum texto. A redação `✖ … / fix: …` é da CLI. O motor usa `thiserror` e a CLI usa `anyhow` só no caminho do `internal error`.
- **Progresso:** o motor nunca imprime. Emite eventos `Progress` tipados num canal, e a CLI renderiza linhas (sem rodapé no M1). Diagnóstico interno vai por `tracing`, visível com `SURT_LOG`.
- **Parada:** `StopSignal` com níveis `None`/`Graceful`/`Force`. O primeiro Ctrl-C/SIGHUP/fechamento de console chama graceful e o segundo chama force. Um cancel request gravado no SQLite é lido pelo `drive` no próximo tick.

### Store

- Todo SQL fica num único módulo. O schema é `0001_initial.sql` do plano, sem alteração e sem comentários: 17 tabelas, STRICT, ids UUIDv7 TEXT, timestamps INTEGER em segundos Unix gravados pelo Rust.
- Abertura: WAL + `synchronous=FULL`, migrações sob `db.lock`, e um banco mais novo que o binário recusa (`SchemaTooNew`).
- Há **um método por checkpoint transacional** de [I](../../docs/architecture/I-recovery.md#transactional-checkpoints), não um repositório por tabela. Cada método é uma transação `BEGIN IMMEDIATE` com retry em `SQLITE_BUSY` que grava suas linhas e seu `event` juntos. Nenhum handle de transação sai do módulo.
- `load_run_graph` carrega tudo o que o scheduler lê, e `drive` recarrega o `RunGraph` a cada tick (~1 s), então Approvals e cancel requests de outro processo aparecem sem cache.

### Scheduler puro

- Dentro de `drive`, decidir é separado de fazer: `decide(&RunGraph, &Ctx) -> Vec<Action>`, com ações `StartIteration`, `ExpandItems`, `Launch`, `RunOperation`, `Block`, `FinishRun`, `Return(DriveOutcome)` e `Wait`.
- `RunGraph`, `Action`, `Ctx` e os enums de estado não importam `sqlx` nem o módulo `git`. `store` depende deles, nunca o contrário (exceção ao DIP registrada no [ADR 0012](../../docs/adr/0012-agent-runtime-is-the-only-trait.md)).

### Única trait: `AgentRuntime`

- Usa `async` nativo, não é object-safe, e `drive` recebe `&impl AgentRuntime`.
- `ClaudeCodeRuntime` e `FakeRuntime` são públicos em `surt-engine::runtime`. O runtime lança, acompanha e para uma Execution. Não resolve inputs, não publica Revisions e não grava no SQLite.
- `RuntimeError` carrega o `FailureClass` (`transient`, `not_retryable`, `retryable`) classificado pelo adaptador. `drive` grava esse valor na Execution, e ele nunca atravessa a fachada como erro.
- Store, Git, Operations e relógio **não** têm trait: são reais nos testes, e timers usam `tokio::time::pause()`.

### Workflows e Agents embutidos

- O parser aceita os cinco `kind` (`agent`, `approval`, `operation`, `loop`, `per_item`) com `deny_unknown_fields`, sem expressões. Condições só comparam Outcome, e `needs` é explícito.
- A validação estática roda contra os agents e o registro de Operations. O root congela o fecho de definições, agents e config com identidade SHA-256.
- O `Catalog` é uma tabela explícita com `include_str!`, montada pelo crate `surt`. O motor embute só suas migrations e os schemas dos artefatos de Operations (`base`, `check-report`, `conflict`, `integration`). Os schemas de agent (`manifest`, `commit`, `feedback`, `delivery`) viajam com seus Agents.
- `feature.toml` no M1 **não tem o step `merge`** (`git.merge_into_base` não é implementado). `[cleanup] on_success` só aceita `"clean"`.
- Os cinco Agents têm o mesmo perfil embutido: `claude-code`, `claude-sonnet-5-5`, effort `high`, auth `subscription`, `dontAsk`, com tools por papel conforme [F](../../docs/architecture/F-agent-contract.md#permissions-and-tools). Nenhum limita turnos nem custo.
- O nome do Subject é `YYYYMMDD-<slug>` com sufixo `-N` automático. A data vem do relógio local via `jiff`, e o nome segue as regras de nome portável.

### Artefatos e contexto

- Os bytes das Revisions são blobs SHA-256 no SQLite, com publicação atômica. Uma Approval guarda id + hash.
- Working Copy existe só para artefatos com Approval Gate, em `.surt/features/<feature>/`. A detecção de edição usa SHA-256 completo contra o baseline, nunca mtime.
- A pendência no Working Copy (`pending_sha256`/`pending_revision_id`) é gravada na transação de publicação e promovida depois da escrita do arquivo.
- Pinning e revalidação seguem [H](../../docs/architecture/H-artifact-storage.md#revalidation): editar uma Revision aprovada e não consumida gera uma Revision não aprovada e a recusa dos drivers; editar uma Pinned Revision é recusado.
- Context Assembler: inputs por Execution em `.surt/runtime/executions/<id>/` e o arquivo do item como `task.md` somente leitura. Cada Input leva um rótulo com a Iteration e o step que o produziram, e a partir da segunda Iteration o contexto diz por que ela começou.
- A validação de entrega é única para o Fake, o Claude Code e as edições humanas de Item List: envelope `{outcome, outputs}`, caminhos relativos sem escape nem link, formatos e schemas (draft-07), e checks de consistência registrados pelo workflow embutido (evidência Git do commit do Builder, `reviewed_commit` = `commit.sha`, `approved` sem finding bloqueante).

### Git e Operations

- O CLI `git` roda por argv com `LC_ALL=C`, versão mínima 2.38 e `git.lock` com backoff para locks de terceiros.
- Cinco Operations num `enum` fechado: `git.create_run_branch`, `git.create_child_worktree`, `checks.run`, `git.integrate` e `git.snapshot_run_branch`. Toda Operation segue "lê estado → compara com SQLite → age", então a próxima Attempt é sua própria reconciliação.
- `git.create_child_worktree` e `git.integrate` são serializados por root run (mutex no processo).
- Integração ([ADR 0007](../../docs/adr/0007-integrate-reviewed-sha-as-merge-commit.md)): `merge-tree --write-tree` → `commit-tree -p T -p R` datado por `step_run.started_at` → re-check dos Checks se `T ≠ S` → Intention `ref_move` → `update-ref` com valor antigo esperado → confirmação.
- Worktrees ficam em `~/.surt/worktrees/<project-id>/<feature>/<key>`, criadas com `worktree add --lock`. No Windows os Builders recebem `core.longpaths=true` via `GIT_CONFIG_*`.
- Toda Attempt numa worktree passa antes pela preparação ([ADR 0010](../../docs/adr/0010-every-attempt-starts-from-a-clean-worktree.md)): localizar → remover locks próprios → salvar sobras em `refs/surt/salvage/<execution-id>` com índice temporário → limpar operações em andamento → `checkout --force -B` + `clean -fd` → verificar. O fluxo normal e a recuperação usam o mesmo código.

### Processos e concorrência

- Um Driver por root run, garantido por lock de arquivo em `.surt/runtime/locks/<root-run-id>.lock` via `File::lock` ([ADR 0002](../../docs/adr/0002-file-locks-over-sqlite-leases.md)), sem lease nem heartbeat.
- A identidade de processo é pid + start time nativo + boot id. Órfãos são parados, nunca adotados.
- Árvores de processo usam `process-wrap` (process group / Job Object com kill-on-close), com `PR_SET_PDEATHSIG` no Linux. `stop_grace_period` é fixo em 10 s.
- `max_concurrent_agents` é um semáforo **por Driver**, lido antes de cada lançamento. `execution.slot` continua sendo gravado.

### Recovery, cancelamento e cleanup

- A Recovery roda dentro de todo comando que dirige, nunca como modo separado. São os sete passos de [I](../../docs/architecture/I-recovery.md#what-a-driving-command-does-when-it-takes-the-lock), todos no M1, inclusive Salvage Refs e a recusa por Integration Branch movida ou com checkout.
- No M1 uma Transient Failure **bloqueia na hora**: Execution `failed` com `failure_class = 'transient'` (não gasta Attempt), Step Run `blocked` com `blocked_reason = 'not_retryable'`, exit 4, e a hora de reset aparece na mensagem quando o stream a informa. Não há pausa nem backoff.
- `cancel` sem Driver toma o lock, roda os passos 3–4 e registra a Cancellation. Com Driver vivo, grava o cancel request e o Driver sai com 5.
- O cleanup roda após o estado terminal, sob o lock, e é idempotente. Se falhar, fica pendente (`cleaned_at` NULL) e é retomado pelo próximo `feature`/`plan`/`start`/`cancel` da Feature. A tabela "o que cada fim remove" de [I. Cleanup](../../docs/architecture/I-cleanup.md#what-each-ending-removes) vale para `succeeded`/`clean` e `cancelled`.

### Claude Code Runtime

- O lançador usa o argv de [F](../../docs/architecture/F-agent-contract.md#claude-code-launcher-and-compatibility) (`-p`, `--session-id` novo, `--no-session-persistence`, `--output-format stream-json --verbose`, modelo e effort completos, `--permission-mode`, tools e regras explícitas, `--json-schema` gerado, `--append-system-prompt-file`, `--settings` gerado, MCP vazio estrito, `--disable-slash-commands`, e `--safe-mode --setting-sources user` para subscription). O contexto vai pelo stdin, que é fechado em seguida.
- O ambiente do filho é montado por **allowlist** mais as variáveis que desligam auto-update, memória e CLAUDE.md. Regras de path de escrita usam o namespace `Edit(...)` com prefixo `//` para raiz absoluta POSIX.
- O parser lê o stream até o processo terminar e drenar, ignora tipos e campos desconhecidos e não presume que `system/init` venha primeiro. Para aceitar, exige os cinco critérios de [F](../../docs/architecture/F-agent-contract.md#runtime-success-and-failure); `subtype = "success"` sozinho não prova nada.
- A classificação de falha lê `assistant.error` e `terminal_reason`. Uma categoria desconhecida é `retryable`.
- Stream e stderr vão para `.surt/logs/<feature>/<execution-id>/`.
- A versão mínima é 2.1.301, e a versão real é gravada em cada Execution.

### CLI do M1

- Dez comandos: `init`; `feature "<description>"` com `--name`, `--base`, `--new`; `feature <feature>`; `plan`; `start`; `approve [--rev N]`; `reject -m [--rev N]`; `cancel [--yes]`; `status [<feature>] [--all]`; `show <feature> [<artifact>] [--rev N] [--task <key>]`.
- Comandos e flags cortados não existem no clap.
- A tabela de próximos comandos (`approve-spec` → `plan`/`feature`, `approve-tasks` → `start`/`plan`) mora no crate `surt`.
- `status <feature>` imprime o caminho do log e o comando de retomada de cada step bloqueado.
- Sem TTY nada é perguntado. `cancel` sem `--yes` recusa com 3.

### Ordem de construção

Cada fatia é um PR em `main` com CI verde nos três SOs e um "pronto" demonstrável ([K](../../docs/architecture/K-roadmap.md#build-order)):

| Fatia | Pronto quando |
|---|---|
| S1 Esqueleto: workspace, toolchain, CI, `platform`, `store`, `Project::open`, `init` | `surt init` funciona e é idempotente nos três SOs |
| S2 Workflow: parser, validação, freeze, `Catalog`, `create_run`, nome do Subject | `surt feature "add login"` cria o Subject e um run `pending` e para sem dirigir |
| S3 Motor + Fake: `decide`, `drive`, `artifacts`, `context`, trait + Fake + `test-support`, `approve`/`reject` | spec → gate → reject → spec rev 2 → approve → `plan` → gate, com o Fake, pelo binário |
| S4 Git: `git`, cinco Operations, preparação/salvage de worktree, `start` | E2E do brief verde (sem `kill`) + os três cenários Git |
| S5 Recovery e fim: `reconcile`, `ref_move`, pendências do Working Copy, fault points, smoke de `kill`, cleanup com retry, `cancel`, `status` detalhado, `show` | a suíte inteira da estratégia de testes passa |
| S6 Claude Code (em paralelo a partir de S3): lançador, allowlist, parser, classificação, árvores de processo, fixtures | o parser passa contra os fixtures e o smoke manual em macOS passa |

## Testing Decisions

### O que é um bom teste aqui

- Um teste afirma **comportamento externo** visto de um seam: exit code, linhas de stderr que fazem parte do contrato, estado final no Git (refs, merge commits, worktrees) e no SQLite (estados de run/step/execution, Revisions, Approvals, `cleaned_at`). Não afirma ordem de chamadas internas, nomes de funções privadas nem formato de structs intermediárias.
- Store, Git e relógio são **reais**: SQLite em arquivo num tempdir (nunca in-memory), repositório Git temporário por teste, timers com `tokio::time::pause()` e funções puras recebendo `now`. A regra é a de `rules/architecture.md`: testar o domínio sem I/O e os adaptadores nas bordas.
- Nenhum `sleep` real, e todo teste é determinístico nos três SOs. A Execution "que espera até ser parada" do Fake substitui temporização.
- Testes do motor usam **workflows sintéticos**. O `feature.toml` só é exercitado pelo binário.
- Cada teste cobre sucesso, erro e borda, conforme `rules/rust.md`.

### Seams (confirmados)

1. **Binário `surt`, o seam principal.** O E2E do brief roda aqui: `init` → `feature` → spec → reject → spec rev 2 → approve → `plan` → approve → `start` com 2+ Tasks, uma com review reprovado e novo Builder → integração → Shipper → cleanup → verificação do estado final lendo Git e SQLite diretamente. Também cobre cada exit code (0, 2, 3, 4, 5, 130) e a garantia de que `status`/`show` não escrevem nada (hash do banco e do Working Copy antes e depois). O Fake é selecionado por `SURT_RUNTIME` com o caminho de um roteiro TOML, só com a feature `test-support`. Teto de 60 s por SO.
2. **Fachada `Project`.** Testes de integração do motor com roteiro tipado em Rust, indexado por chave lógica do step + Iteration + Attempt (`Script::new().on("build", 1, 1, Behavior::Commit)`). Cobre:
   - fault points no store nos **oito checkpoints** (run criado, Iteration/expansão, Execution iniciada, processo lançado, Intention, Execution aceita, Working Copy escrito, Execution encerrada de outro modo): para cada um, falhar logo após o commit e conferir que o próximo `drive` termina o run sem duplicar efeito;
   - o **smoke de `kill` real**: matar o processo durante uma Execution em espera do Fake; a retomada termina o run, com a Execution morta `interrupted` e um novo Attempt;
   - os **três cenários Git**: merge limpo, conflito (volta ao Builder com `conflict`) e Checks falhando (pula review, Builder lê `check-report`);
   - invariantes do schema: ≤1 root ativo por Subject e ≤1 Execution viva por Step Run, garantidos por CHECK e índice único;
   - segundo Driver no mesmo run recusa, e `kill -9` libera o lock;
   - pinning e revalidação, detecção de edição e edição inválida de Item List;
   - cleanup completo, cleanup pendente após falha de remoção e retomada pelo próximo comando.
3. **`decide(&RunGraph, &Ctx)`, testes de tabela.** Cobre as regras duras de G: 3 Attempts → `blocked`; 5 Iterations sem gate → `blocked`; Loop com Approval Gate sem limite; `interrupted` e `transient` não gastam Attempt; `not_retryable` bloqueia na primeira; orçamento renovado ao desbloquear; dependentes de Task bloqueada esperam e independentes seguem; run só termina `succeeded`/`cancelled`; `Return` quando nada roda sem humano.
4. **`ClaudeCodeRuntime` contra fixtures reais.** Os streams capturados (sucesso, falha de auth, modelo inexistente, SIGTERM 143 sem `result`) são copiados da branch `research/claude-code-cli-headless` para os fixtures de teste. Os testes conferem parser, critérios de aceitação, classificação de falha e montagem de argv e ambiente (allowlist, regras `Edit(//…)`). O Claude Code real nunca roda na CI.

### Módulos testados

| Módulo | Seam |
|---|---|
| `workflow` (parse, validação, freeze) | `Project` (create_run com definições sintéticas válidas e inválidas) |
| `drive` / `decide` | `decide` (tabelas) e `Project` |
| `store` | `Project` (checkpoints, fault points, invariantes) |
| `artifacts`, `context` | `Project` |
| `git`, `operations` | `Project` (três cenários Git, repositório real) |
| `reconcile`, `cleanup` | `Project` (fault points, smoke de `kill`) |
| `runtime::claude_code` | fixtures |
| `runtime::fake` | indiretamente, por todos os outros |
| `platform` | `Project` nos três SOs (locks, identidade de processo, remoção segura) |
| crate `surt` (comandos, render, exit) | binário |

### Prior art

- O código ainda não tem testes; o único crate é o `ferris-says` descartável. A referência é o plano: a estratégia de testes no [ticket 16](../surt-architecture/issues/16-estrategia-de-testes.md), os streams reais e o lançador validado no [ticket 19](../surt-architecture/issues/19-fixture-stream-real-claude-code.md) (branch `research/claude-code-cli-headless`), os SQL e TOML do plano (que já rodam em `sqlite3` e fazem parse) e os mocks de terminal em A, G, I e J, que são o contrato das mensagens.

### CI

- `.github/workflows/ci.yml` com matriz macOS/Linux/Windows. Cada SO roda `cargo fmt --all -- --check`, `cargo clippy --all-targets --features test-support -- -D warnings` e `cargo test --features test-support`, com Git ≥ 2.38 pinado por runner.
- O smoke com Claude Code real é manual, só em macOS, seguindo o checklist `docs/smoke-claude-code.md` (a escrever no S6): num repositório de brinquedo, uma Feature com duas Tasks chega a `succeeded`.

## Out of Scope

- **Comandos e flags do M2:** `stop` (Ctrl-C cobre), `inspect`, `logs` (`status` mostra o caminho do log), `restore` (apagar o arquivo o recria), `delete`, `--new-run`, `--json`, `--run`, `--iter`, `--file`, rodapé de TTY, linha de 5 minutos sem TTY, símbolos ASCII.
- **Mecanismos do M3:** pausa e backoff de Transient Failure (no M1 ela bloqueia), vagas de agente entre processos (no M1 é semáforo por Driver), `~/.surt/config.toml` global.
- **Mecanismos do M4:** `merge_into_base = "ff-only"`, o step `merge`/`git.merge_into_base` e `--merge`/`--no-merge`; `on_success = "keep"`; overrides em `.surt/agents/`.
- **Smoke real em Linux e Windows** e o modo API key (`--bare`): sem validação manual no M1. O que a CI não pegar vira bug.
- **Testes fora da v1:** toda linha das tabelas de recuperação, Ctrl-C duplo, migrations concorrentes, matriz de versões do Git, ASCII sem UTF-8, snapshot de todas as mensagens, virada de data no slug.
- **Fora do mapa inteiro:** interface gráfica; daemon e `--detach`; integrações externas (GitHub, Linear, Jira, push, PR); um segundo runtime de IA real; workflows próprios do projeto ou override dos embutidos; cancelar uma Task isolada com entrega parcial; coordenar rate limit entre Drivers; redação de segredos e rotação de logs; remover a worktree de cada Task logo após integrá-la; infra cloud.

## Further Notes

- Nenhum corte do M1 exige migration: `0001_initial.sql` já tem `cleaned_at`, `execution.slot`, `failure_class`, `ref_move` e as colunas de stop/cancel request. M2–M4 entram sem mudar o schema.
- **Done do M1:** (1) suíte verde na CI nos três SOs, com o E2E abaixo de 60 s por SO; (2) smoke manual com Claude Code real em macOS passando.
- Para quem implementa, a referência é o plano: [A–L](../../docs/architecture/), [ADRs 0001–0012](../../docs/adr/), o vocabulário em [`CONTEXT.md`](../../CONTEXT.md) e as regras em [`rules/`](../../rules/). Desvio deliberado de uma regra fica registrado no formato `Exceptions:` das regras, como já fazem [D](../../docs/architecture/D-database.md#rule-exceptions) e o ADR 0012.
- Os schemas em `docs/architecture/schemas/` mudam para o crate dono: os de Operation para `surt-engine` e os de agent para o Agent correspondente em `surt`. `docs/architecture/migrations/0001_initial.sql` é copiado sem alteração.
- S6 pode correr em paralelo a S4/S5 assim que S3 fixar a trait `AgentRuntime`.
- Depois do M1: M2 Operar e depurar → M3 Rodar sem babá → M4 Ajustar ao projeto. A v1 é o fim do M4.
