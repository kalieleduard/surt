# Estratégia de recuperação e reconciliação

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: resolved
Blocked by: 05, 09, 11, 18
Assignee: Kaliel

## Question

Como o Surt retoma qualquer workflow — dias depois, após crash, reboot ou kill — sem duplicar trabalho nem confiar em sessões Claude antigas?

Decidir: em que pontos há checkpoint transacional; padrão de intenção → efeito → confirmação para operações externas (git, filesystem, processo); detecção de execuções órfãs (PID + start time? lease expirado?) e o que fazer com seus resultados parciais (worktree suja, commit feito mas não registrado); reconciliação de worktrees e branches contra o SQLite; o que `surt resume` faz passo a passo; como attempt incrementa após falha técnica e limite de retries; erros de rede/rate limit do Claude Code; idempotência de cada operação nativa.

Saída: seção I e a parte de recuperação da seção G.

## Context

Surgiu de [Mecânica de worktree/merge no Git e estados parciais](04-git-worktree-merge-estados-parciais.md): comandos git não são idempotentes por si — a reconciliação é "ler estado → comparar com registro → agir"; usar a tabela de 21 estados parciais do documento como base; regra CAS de `update-ref` (novo → feito, antigo → refazer, outro → parar); worktree `locked initializing`, `index.lock` órfão e marcadores de merge/rebase como casos obrigatórios; risco de `git gc` podar worktrees ausentes após 3 meses em runs retomados dias depois.

De [Modelo de processo e controle de concorrência](05-modelo-de-processo-e-concorrencia.md), já decidido: órfã = Execution em estado de execução cujo lock do root run está livre; órfãos **nunca são adotados** — o comando que dirige mata o grupo sobrevivente (identidade pid + start time + boot id) e cria novo Attempt. Não há `surt resume`: reexecutar `surt feature`/`plan`/`start` retoma. O primeiro Ctrl-C deixa a Native Operation em andamento terminar; só o segundo Ctrl-C (ou kill -9/crash) deixa estado parcial para reconciliar. Falta aqui: o que fazer com a worktree suja/commit não registrado que o órfão ou a Interruption deixaram.

De [Modelo de integração Git](09-modelo-de-integracao-git.md) ([A. Git Integration](../../../docs/architecture/A-git-integration.md)): estados parciais a reconciliar por Operation: `git.create_integration_branch` (branch existe?), `git.create_task_worktree` (branch sem worktree, worktree `locked initializing`), `git.integrate` (ref = M → feito; = T → refazer; outro → parar; worktree da Task deixada em `detach M` no meio do re-check; commit M sem ref é lixo inofensivo), `git.merge_into_base` (`merge --ff-only` interrompido na worktree do usuário). Integration Branch checada em outra worktree = falha técnica, retentada no próximo comando; ponta divergente da gravada = **para sem adotar**, com a instrução `git branch -f`. Decidir aqui o estado do run nesse caso e como a retomada confirma que a ponta foi restaurada.

De [Máquina de estados do loop Builder ↔ Reviewer](10-loop-builder-reviewer.md) ([G. State Machines](../../../docs/architecture/G-state-machines.md)): Execution `interrupted` (Ctrl-C ou órfão parado pelo próximo Driver) **não conta** para `max_attempts = 3`; `failed` conta. Falhas que retry não resolve (auth, modelo, launch, Integration Branch movida por fora/checada em worktree) levam o Step Run a `blocked` na 1ª ocorrência; decidir aqui quais erros de rede/rate limit esperam em vez de virar Attempt e completar a lista de não-retentáveis. Um driving command que pega o lock move todo `blocked` do run de volta a `running` com orçamento novo. Não existe run `failed` ([ADR 0008](../../../docs/adr/0008-limits-block-instead-of-failing.md)).

De [Schema SQLite](11-schema-sqlite.md) ([D. Database Schema](../../../docs/architecture/D-database.md)): a reconciliação compara Git com `run_branch.tip_sha` (CAS) e `worktree_path`; órfãos são identificados por `execution.pid` + `process_start_time` + `process_boot_id`. O `git.integrate` agora data o merge commit com `step_run.started_at`, então todo Attempt recalcula o mesmo `M`. `synchronous = FULL` garante que um registro de intenção commitado sobrevive a corte de energia; decidir aqui o que se grava antes de cada efeito externo (há `event` para auditoria, mas o engine nunca o lê para decidir; o estado precisa estar em colunas). Classes de falha: `execution.failure_class` ∈ `retryable | not_retryable`, com `error_code` livre; se rede/rate limit precisarem de uma terceira classe ("esperar"), é migration aditiva no CHECK (rebuild da tabela).

## Answer

Resolvido por troca humana em três rodadas (14 perguntas, todas aceitas com a recomendação). Seção completa em [I. Recovery and Reconciliation](../../../docs/architecture/I-recovery.md); parte de recuperação da [G. State Machines](../../../docs/architecture/G-state-machines.md) atualizada; trade-off em [ADR 0010 — Every Attempt starts from a clean worktree; leftovers are salvaged, never continued](../../../docs/adr/0010-every-attempt-starts-from-a-clean-worktree.md). Termos novos no `CONTEXT.md`: **Recovery**, **Intention**, **Salvage Ref**, **Transient Failure**.

Decisões finais:

- **Recuperação não é modo.** Sem `surt doctor`/`repair`/`resume`: todo comando que dirige, ao pegar o lock, faz *preflight → lock → Executions vivas no banco → Intentions → checagem da Integration Branch → desbloqueio → agendamento*. Toda Operation é "ler → comparar → agir", e o próximo Attempt é a reconciliação.
- **Só é aceito o que o Driver registrou terminar.** Delivery válida de uma Execution cujo fim não foi gravado é descartada (`interrupted`, sem contar Attempt), sem revalidação.
- **Intention em coluna antes de todo efeito externo; a recuperação só lê e compara, nunca recalcula.** Criar branch/worktree: a linha `run_branch` é gravada antes (`start_sha` = `tip_sha` = alvo). Mover ref: nova tabela **`ref_move`** (`step_run_id` PK, `ref_name`, `old_sha`, `new_sha`, `worktree_path`), que cobre `git.integrate` e `git.merge_into_base` com uma regra: ref = novo → confirma; = antigo → refaz; outro → para (integrate bloqueia `integration_branch_moved`; merge_into_base dá `skipped`). Substituiu a ideia de `run_branch.pending_tip_sha`, porque a Base não é `run_branch`.
- **Working Copy:** `working_copy_file.pending_sha256`/`pending_revision_id` gravados na publicação, promovidos depois da escrita (temp + rename); a varredura distingue escrita feita, escrita não feita e edição humana na janela, que vira Revision humana depois da do agente.
- **Preparo da worktree antes de todo Attempt** (agente ou Operation, com crash ou não; ADR 0010): alvo único = branch do run em `run_branch.tip_sha` (último tip aceito, gravado na transação de publicação); remove locks da worktree, salva tudo o que diverge num commit em `refs/surt/salvage/<execution-id>` (índice temporário), limpa marcadores (`--quit`), `checkout --force -B` + `clean -fd` (ignorados preservados). Commits humanos na Task Branch também vão para o salvage. O aviso no terminal só aparece para sobra inesperada.
- **Estados de worktree consertados sozinhos** (branch sem worktree, `locked initializing`, diretório apagado, repo movido → `worktree repair`, metadado apagado → renomeia `.orphaned-<ts>` e recria); só bloqueia quando o caminho é de outro dono. Worktrees criadas com `--lock` contra o `gc` de 3 meses.
- **Locks de Git:** o Surt só apaga lock provadamente seu com dono morto (worktree de run sem Execution viva; `<ref>.lock` de `surt/…` sob `git.lock`); os de fora viram `failed` `retryable` com o arquivo nomeado.
- **Integration Branch movida ou checada:** o comando recusa na partida sem mudar estado (`status` segue `INTERRUPTED`); durante o run, a Operation bloqueia (`not_retryable`), e a restauração é confirmada por ref = `tip_sha`, sem coluna extra.
- **`merge_into_base` interrompido na worktree do usuário:** o Surt nunca escreve nela; o `ref_move` com `worktree_path` fica, e o Step Run bloqueia (`merge_into_base_interrupted`) com as instruções *finish* / *undo*.
- **Transient Failure:** terceira `failure_class = 'transient'` (`rate_limit`, `overloaded`, `server_error`, rede), não conta Attempt e pausa **todos** os lançamentos de agente do Driver até o reset informado ou backoff 1→2→4→8→15 min, sem teto (foreground, Ctrl-C). `authentication_failed`, `billing_error`, `model_not_found` e `invalid_request` são `not_retryable`; categoria desconhecida é `retryable`. Operations nunca são `transient`.
- **Sem Driver:** `surt cancel` roda as etapas de órfãos e Intentions antes de cancelar (o histórico diz onde a branch está); `surt status` só lê e avisa de órfão vivo.
- Risco residual documentado: queda entre o spawn e o registro do pid deixa um processo não identificado no macOS; ele não afeta o resultado (aceite amarrado a SHAs registrados).

Mudanças no schema (a migration 0001 ainda não foi lançada, então foi editada): `ref_move` (18 tabelas), colunas pending em `working_copy_file`, `'transient'` no CHECK de `execution.failure_class`, novos kinds de `event`. Validação: a migration roda em `sqlite3` 3.51 com `-bail`; os CHECKs novos recusam `old_sha = new_sha`, pending pela metade e classe desconhecida.

Notas de contexto acrescentadas a [Segurança do cleanup e do ciclo de vida](13-cleanup-e-ciclo-de-vida.md), [Superfície e UX da CLI](14-superficie-e-ux-da-cli.md), [Estratégia de testes](16-estrategia-de-testes.md) e [Capturar stream real do Claude Code como fixture](19-fixture-stream-real-claude-code.md). Nenhum ticket novo.
