# 03: `surt init` and project discovery

Spec: [Milestone 1](../spec.md) (stories 1–9, 95–100, 103) · Slice: S1 Skeleton · Plan: [J](../../../docs/architecture/J-cli.md#surt-init-and-finding-the-project), [A. Git Integration](../../../docs/architecture/A-git-integration.md)

**What to build:** a developer runs `surt init` in any repository and gets a `.surt/` project at the toplevel of the main worktree, with clear refusals when prerequisites are missing. Every other command finds the project from any subdirectory through `Project::open`, and refuses outside one. This ticket also establishes the CLI's error shape, exit-code mapping and diagnostics that every later command reuses. Slice S1 is done when `surt init` works and is idempotent on the three OSes.

**Blocked by:** 02 (Store opens the schema)

**Status:** ready-for-human

- [x] `surt init` creates `.surt/` at the toplevel of the repository's main worktree, opens the database, writes a `.surt/.gitignore` that versions only `config.toml` and `agents/`, and a commented `config.toml` with the defaults (`max_concurrent_agents = 3`, `max_attempts = 3`, `max_iterations = 5`, `merge_into_base = "never"`, an example `[[checks]]`)
- [x] `surt init` refuses with a fix when Git < 2.38 or no user identity is configured
- [x] Rerunning `surt init` is idempotent and never overwrites `config.toml` or `agents/`
- [x] Any other command outside an initialized project exits `3` and points at `surt init`; inside a Surt-managed worktree under `~/.surt/worktrees/` it refuses
- [x] Unknown keys in `config.toml` are errors; `merge_into_base = "ff-only"` refuses with exit `3`
- [x] Errors follow J's shape (`✖` line, at most two context lines, one to three labelled copyable commands); progress and errors go to stderr, data to stdout
- [x] An unexpected error prints `✖ internal error: …` and suggests `SURT_LOG=debug`; `SURT_LOG` enables diagnostics on stderr; `NO_COLOR` disables colour
- [ ] Binary tests on the three OSes cover first init, rerun, missing identity, outside-project refusal and an unknown config key

## Comments

- 2026-10-18 (orchestrator): Implemented and green on macOS. Open criterion: binary tests on Linux and Windows — waits for the first CI run of the matrix (needs the branch pushed). Merged into `integration/milestone-1` at f5a0ca6; code-review fixes at 931fc99.
