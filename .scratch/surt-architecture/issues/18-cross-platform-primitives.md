# Cross-platform primitives (macOS, Linux, Windows)

Map: [Surt — Architecture and implementation plan](../map.md)
Type: research
Status: resolved

## Question

Which OS primitives does Surt need, and how does each one behave on macOS, Linux and Windows, with which crates?

Survey: supervising child processes and killing the whole tree (process group + `killpg` × Job Objects) with Tokio; detecting whether a PID is still the same process (start time) on each OS; signals × `CTRL_BREAK_EVENT`; advisory × mandatory file locks (`fs4`/`fd-lock`) and SQLite WAL locks on Windows; symlinks, junctions and reparse points when cleaning up directories safely (safe `remove_dir_all`, `std::fs::remove_dir_all` and CVE-2022-21658); path canonicalization (`dunce`, `\\?\`), long paths; home/data directory per OS for `~/.surt/worktrees` (`dirs`/`directories`); local date for feature names; single-binary distribution per OS.

Findings: `docs/research/cross-platform-primitives.md` on the `research/cross-platform-primitives` branch.

## Answer

Full findings: `docs/research/cross-platform-primitives.md` on the `research/cross-platform-primitives` branch (commit `bffcfa4`). Verified empirically only on macOS (rustc 1.99); Linux and Windows only from documentation. **Premise correction:** the current `claude` is a native binary (not Node), but it still spawns children (Git Bash/PowerShell, hooks, MCP).

Facts that constrain the design (not decisions):
- `kill_on_drop`/`Child::kill` only reach the direct child (a grandchild survived in the test). Unix requires `process_group(0)` + killing the group; Windows requires a Job Object. `process-wrap` 10.x is the maintained successor of `command-group`, but it does not escalate SIGTERM→SIGKILL on its own.
- Windows has no SIGTERM; the only "graceful" option is `CTRL_BREAK_EVENT` with `CREATE_NEW_PROCESS_GROUP` and a shared console.
- PID identity needs a native start time + boot identity (Linux `/proc/<pid>/stat` + `boot_id`; macOS `proc_pidinfo` + `kern.boottime`; Windows `GetProcessTimes`); `sysinfo` only gives seconds.
- `std::fs::File::lock`/`try_lock` stable since Rust 1.89 (flock on Unix, mandatory `LockFileEx` on Windows), released when the process dies, not inherited by children. The lock must live in a **separate file** from the DB/`-wal`/`-shm` (SQLite uses POSIX locks that an outside `close()` drops).
- `std::fs::remove_dir_all` (≥1.58.1) does not follow links below the root and treats junctions as symlinks on Windows, but it is **not idempotent**, can fail under concurrent writes and does not protect against an ancestor directory being swapped after the check.
- `canonicalize` returns `\\?\` on Windows (`dunce` strips it); APFS/NTFS are case-insensitive by default — naive prefix comparison fails. Windows reserved names (CON, NUL, COM1…), `<>:"/\|?*` and a trailing dot/space affect the feature's `description`.
- Per-OS directories via `dirs`/`directories` (macOS with a space in the path; Windows in Roaming); Claude Code already uses `%USERPROFILE%\.claude`, a precedent for a dot-folder. `time` may return `IndeterminateOffset`; `chrono`/`jiff` are alternatives.
- Distribution: static musl on Linux; MSVC needs `+crt-static`; macOS requires signing + notarization; SmartScreen warns about a binary without reputation; `cargo-dist` automates part of it.

Open (depend on a real Windows machine): how `claude.exe` reacts to Ctrl+Break, process-wrap's `wait` on Windows, Git for Windows' `core.longpaths` — recorded in the map's fog.
