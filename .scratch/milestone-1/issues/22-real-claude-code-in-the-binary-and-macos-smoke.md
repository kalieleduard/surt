# 22: Real Claude Code in the binary and the macOS smoke

Spec: [Milestone 1](../spec.md) (stories 9, 101, 110–111, 118) · Slice: S6 Claude Code · Plan: [F](../../../docs/architecture/F-agent-contract.md), [A. Process Model](../../../docs/architecture/A-process-model.md), [K](../../../docs/architecture/K-roadmap.md)

**What to build:** the release `surt` binary drives Features with the real Claude Code, stops agents together with their whole process tree on every OS, and refuses an outdated Claude Code with an actionable message. A written checklist makes "Milestone 1 done" a repeatable check: in a toy repository on macOS, a Feature with two Tasks reaches `succeeded`.

**Blocked by:** 15 (`surt start` delivers a Feature), 21 (Claude Code runtime against fixtures)

**Status:** ready-for-agent

- [ ] Without `SURT_RUNTIME`, the binary uses `ClaudeCodeRuntime`
- [ ] Agent processes run under `process-wrap` (process group on Unix, Job Object with kill-on-close on Windows, `PR_SET_PDEATHSIG` on Linux); stopping sends a graceful signal, waits the fixed 10 s grace period, then kills the tree
- [ ] Claude Code missing or older than 2.1.301 is a launch error that blocks at once with an upgrade message; the actual version is recorded on each Execution
- [ ] Each Execution's stream and stderr are written to the Feature's log directory under `.surt/logs/`
- [ ] A manual smoke checklist for macOS is written in `docs/`, covering setup of a toy repository, the full flow with two Tasks, and the expected final state
- [ ] The manual smoke passes on macOS and the result is recorded on this ticket
