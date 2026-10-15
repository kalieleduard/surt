# Primitivas multiplataforma (macOS, Linux, Windows)

Map: [Surt — Arquitetura e plano de implementação](../map.md)
Type: research
Status: claimed (research subagent)

## Question

Quais primitivas de SO o Surt precisa e como cada uma se comporta em macOS, Linux e Windows, com quais crates?

Levantar: supervisão de processos filhos e matar a árvore inteira (process group + `killpg` × Job Objects) com Tokio; detectar se um PID ainda é o mesmo processo (start time) em cada SO; sinais × `CTRL_BREAK_EVENT`; file locks advisory × mandatory (`fs4`/`fd-lock`) e locks em SQLite WAL no Windows; symlinks, junctions e reparse points ao limpar diretórios com segurança (`remove_dir_all` seguro, `std::fs::remove_dir_all` e CVE-2022-21658); canonicalização de paths (`dunce`, `\\?\`), paths longos; diretório home/dados por SO para `~/.surt/worktrees` (`dirs`/`directories`); data local para nomes de feature; distribuição de binário único por SO.

Findings: `docs/research/cross-platform-primitives.md` na branch `research/cross-platform-primitives`.
