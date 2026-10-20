# 06: The Claude Code runtime forks the previous session

Spec: [Milestone 6](../spec.md) (What the runtime already showed; Conversation: Launcher, Restart) · Plan: [F. Claude Code launcher](../../../docs/architecture/F-agent-contract.md#claude-code-launcher-and-compatibility)

**What to build:** the real launcher acts on a continuation: `--resume <previous> --fork-session --session-id <new>` with the delta on stdin, and a fresh relaunch when the continued launch never initializes.

**Blocked by:** 05

**Status:** resolved

- [x] `"loop"` Agents never get `--no-session-persistence`; with a continuation the argv adds `--resume <session> --fork-session` next to the new `--session-id`, stdin is the delta context; `"none"` argv is byte-for-byte today's
- [x] A continued launch that ends without `system/init` is relaunched fresh in the same Execution with the full context; the report says `transcript_missing` when the result's `errors` contain `No conversation found`, else `continuation_refused`; a failure after `system/init` is classified as today
- [x] Logs keep both launches' streams under the Execution's log directory without overwriting
- [x] Tests: argv for fresh, continued and `"none"` launches (the existing never-resumes test is scoped to `"none"`); the Unix stub `claude` replays the probe's `No conversation found` result and the runtime restarts; on Windows the argv tests run, the stub tests stay Unix-only as today
- [x] F's launcher section and its "Never use `--resume`" sentence follow ADR 0014
