# Estratégia de testes

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: grilling
Status: open
Blocked by: 07, 10, 12, 19

## Question

Como se testa o Surt sem gastar tokens, incluindo crashes em pontos críticos?

Decidir: desenho do Fake Agent Runtime (roteiros por agente, outputs determinísticos, falhas injetáveis, latência); como simular crash entre intenção e efeito (fault points? matar o processo de verdade em teste de integração?); repositório Git temporário por teste; testes de SQLite (migrations, constraints); o E2E do brief (feature → spec → 2+ tasks → aprovação → builders → reprovação → novo builder → aprovação → shipper → cleanup → verificação do estado) e onde ele roda; testes multiplataforma (CI em macOS/Linux/Windows).

## Context

De [Modelo de processo e controle de concorrência](05-modelo-de-processo-e-concorrencia.md): testar entre processos reais — dois Drivers no mesmo run (o segundo recusa), `kill -9` liberando run lock e vagas, órfão Unix sobrevivendo ao Driver e sendo morto no próximo `start`, Ctrl-C duplo, migrations concorrentes sob `db.lock`, e a escalada SIGTERM/`CTRL_BREAK_EVENT` → kill no Windows.

De [Modelo de integração Git](09-modelo-de-integracao-git.md) ([A. Git Integration](../../../docs/architecture/A-git-integration.md)): testes de integração com repositórios Git reais e descartáveis: conflito textual, merge limpo que quebra os Checks (re-check), determinismo do SHA de `commit-tree` entre Attempts, CAS com ponta movida, branch checada em outra worktree, ff-only com Base movida, worktree suja ou limpa; matriz de versão mínima do Git (2.38) e Windows (longpaths, nomes reservados, `GIT_CONFIG_*`). Os Checks reais são processos filhos com timeout, então o teste de kill de árvore também cobre `checks.run`.

De [Máquina de estados do loop Builder ↔ Reviewer](10-loop-builder-reviewer.md) ([G. State Machines](../../../docs/architecture/G-state-machines.md)): testar com o Fake Runtime: 3 entregas inválidas → `blocked`, reexecução dá mais 3 Attempts (contador continua em 4); Ctrl-C não consome Attempt; falha não-retentável bloqueia na 1ª; 5 Iterations sem merge → Loop `blocked`, sem limite nos Loops com gate; Task bloqueada deixa dependentes `pending` e independentes seguem; commit vazio justificado passa por check/review/integrate; Reviewer da 2ª Iteration recebe `feedback` e, após conflito, `conflict`.

De [Schema SQLite](11-schema-sqlite.md) ([D. Database Schema](../../../docs/architecture/D-database.md)): testes de schema contra arquivo real (não in-memory): cada CHECK/índice único da migration recusando o caso inválido (≤1 root ativo por Subject, ≤1 Execution viva por Step Run, uma decisão por Revision, `rel_path` com `..`, `UPDATE` em `event`), purge de um Subject seguido de `PRAGMA foreign_key_check`, toda query `query_as` exercitada, checksum de migration estável com checkout CRLF (`.gitattributes`), e binário velho recusando DB novo com a mensagem amigável.
