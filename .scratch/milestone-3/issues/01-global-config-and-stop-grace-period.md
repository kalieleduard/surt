# 01: Global `~/.surt/config.toml` and `stop_grace_period`

Spec: [Milestone 3](../spec.md) (stories 40–46) · Plan: [J. Global configuration](../../../docs/architecture/J-cli.md#global-surtconfigtoml-optional), [B. Workspace](../../../docs/architecture/B-workspace.md)

**What to build:** a developer whose Builders run slow test suites can give agents more time to exit cleanly when stopped on their machine. Surt reads `~/.surt/config.toml` (`%USERPROFILE%\.surt\config.toml` on Windows) when it opens a project. Its only key, `stop_grace_period` (default `"10s"`), sets how long every graceful stop of an agent process tree waits before a forced kill: Ctrl-C, a closed terminal, `surt stop`, `surt cancel` with a live Driver, and reconciliation stopping the surviving agents of a dead Driver. With no home directory or no file, the defaults apply. A bad file refuses every command that opens the project.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] `Project::open` loads the global configuration next to the project configuration, using the same home-directory lookup as the worktrees
- [ ] `stop_grace_period` uses the Check `timeout` duration format (positive amount with a unit suffix); unknown keys are errors (`deny_unknown_fields`), including project keys such as `max_concurrent_agents`
- [ ] A parse error is the existing `InvalidConfig` refusal with the global file's path, line and reason; the CLI exits `3` with J's message shape
- [ ] The configured value reaches the Claude Code runtime's stop and reconciliation's stop of stale survivors; both hard-coded 10-second constants are gone
- [ ] The global file is not frozen into runs
- [ ] Config unit tests: defaults with no file, a valid duration, an unknown key, a project key in the global file, a zero or malformed duration
- [ ] Binary test (with `HOME`/`USERPROFILE` pointing at the sandbox): an invalid global file makes a command exit `3` naming the file and line
- [ ] Claude Code stub-process test: a stub that ignores the graceful stop is force-killed after the configured grace period, not after 10 s
- [ ] J records the refusal behaviour; K's "Global `~/.surt/config.toml`" row says it is implemented
