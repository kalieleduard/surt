# Primitivas multiplataforma (macOS, Linux, Windows)

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: research
Status: resolved

## Question

Quais primitivas de SO o Surt precisa e como cada uma se comporta em macOS, Linux e Windows, com quais crates?

Levantar: supervisão de processos filhos e matar a árvore inteira (process group + `killpg` × Job Objects) com Tokio; detectar se um PID ainda é o mesmo processo (start time) em cada SO; sinais × `CTRL_BREAK_EVENT`; file locks advisory × mandatory (`fs4`/`fd-lock`) e locks em SQLite WAL no Windows; symlinks, junctions e reparse points ao limpar diretórios com segurança (`remove_dir_all` seguro, `std::fs::remove_dir_all` e CVE-2022-21658); canonicalização de paths (`dunce`, `\\?\`), paths longos; diretório home/dados por SO para `~/.surt/worktrees` (`dirs`/`directories`); data local para nomes de feature; distribuição de binário único por SO.

Findings: `docs/research/cross-platform-primitives.md` na branch `research/cross-platform-primitives`.

## Answer

Achados completos: `docs/research/cross-platform-primitives.md` na branch `research/cross-platform-primitives` (commit `bffcfa4`). Verificado empiricamente só no macOS (rustc 1.99); Linux e Windows apenas por documentação. **Correção de premissa:** o `claude` atual é binário nativo (não Node), mas ainda gera filhos (Git Bash/PowerShell, hooks, MCP).

Fatos que restringem o design (não são decisões):
- `kill_on_drop`/`Child::kill` só atingem o filho direto (neto sobreviveu no teste). Unix exige `process_group(0)` + kill do grupo; Windows exige Job Object. `process-wrap` 10.x é o sucessor mantido do `command-group`, mas não escalona SIGTERM→SIGKILL sozinho.
- Windows não tem SIGTERM; o único "gracioso" é `CTRL_BREAK_EVENT` com `CREATE_NEW_PROCESS_GROUP` e console compartilhado.
- Identidade de PID precisa de start time nativo + identidade de boot (Linux `/proc/<pid>/stat` + `boot_id`; macOS `proc_pidinfo` + `kern.boottime`; Windows `GetProcessTimes`); `sysinfo` só dá segundos.
- `std::fs::File::lock`/`try_lock` estável desde Rust 1.89 (flock no Unix, `LockFileEx` mandatório no Windows), liberado na morte do processo, não herdado por filhos. Lock deve ficar num **arquivo separado** do DB/`-wal`/`-shm` (SQLite usa POSIX locks que um `close()` externo derruba).
- `std::fs::remove_dir_all` (≥1.58.1) não segue links abaixo da raiz, trata junctions como symlink no Windows, mas **não é idempotente**, pode falhar com escrita concorrente e não protege contra troca de diretório ancestral após a checagem.
- `canonicalize` devolve `\\?\` no Windows (`dunce` remove); APFS/NTFS são case-insensitive por padrão — comparação de prefixo ingênua falha. Nomes reservados do Windows (CON, NUL, COM1…), `<>:"/\|?*` e ponto/espaço final afetam o `description` da feature.
- Diretórios por SO via `dirs`/`directories` (macOS com espaço no path; Windows em Roaming); Claude Code já usa `%USERPROFILE%\.claude`, precedente para dot-folder. `time` pode devolver `IndeterminateOffset`; `chrono`/`jiff` alternativas.
- Distribuição: musl estático no Linux; MSVC precisa `+crt-static`; macOS exige assinatura + notarização; SmartScreen avisa binário sem reputação; `cargo-dist` automatiza parte.

Abertos (dependem de máquina Windows real): reação do `claude.exe` a Ctrl+Break, `wait` do process-wrap no Windows, `core.longpaths` do Git for Windows — registrados na névoa do mapa.
