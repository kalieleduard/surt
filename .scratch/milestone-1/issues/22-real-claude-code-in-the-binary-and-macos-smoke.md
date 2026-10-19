# 22: Real Claude Code in the binary and the macOS smoke

Spec: [Milestone 1](../spec.md) (stories 9, 101, 110–111, 118) · Slice: S6 Claude Code · Plan: [F](../../../docs/architecture/F-agent-contract.md), [A. Process Model](../../../docs/architecture/A-process-model.md), [K](../../../docs/architecture/K-roadmap.md)

**What to build:** the release `surt` binary drives Features with the real Claude Code, stops agents together with their whole process tree on every OS, and refuses an outdated Claude Code with an actionable message. A written checklist makes "Milestone 1 done" a repeatable check: in a toy repository on macOS, a Feature with two Tasks reaches `succeeded`.

**Blocked by:** 15 (`surt start` delivers a Feature), 21 (Claude Code runtime against fixtures)

**Status:** resolved

- [x] Without `SURT_RUNTIME`, the binary uses `ClaudeCodeRuntime`
- [x] Agent processes run under `process-wrap` (process group on Unix, Job Object with kill-on-close on Windows, `PR_SET_PDEATHSIG` on Linux); stopping sends a graceful signal, waits the fixed 10 s grace period, then kills the tree
- [x] Claude Code missing or older than 2.1.301 is a launch error that blocks at once with an upgrade message; the actual version is recorded on each Execution
- [x] Each Execution's stream and stderr are written to the Feature's log directory under `.surt/logs/`
- [x] A manual smoke checklist for macOS is written in `docs/`, covering setup of a toy repository, the full flow with two Tasks, and the expected final state
- [x] The manual smoke passes on macOS and the result is recorded on this ticket

## Smoke results

One line per run of [the checklist](../../../docs/smoke-claude-code.md):

- 2026-10-18 · macOS 26.7.1 (arm64) · Claude Code 2.1.301 · surt 4a9a42f · pass (run and validated by the user)

## Comments

- 2026-10-18 (orchestrator): Implemented and green on macOS with a stub `claude`. Open criterion: the manual smoke with the real Claude Code (`docs/smoke-claude-code.md`), which runs on the user's subscription; record the result in "Smoke results". Merged into `integration/milestone-1` at 1615dee; code-review fixes at 931fc99.
- 2026-10-18 (orchestrator): Resolved. The user ran the manual smoke on macOS and it passed.
