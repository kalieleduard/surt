# 03: The pause is visible from any terminal

Spec: [Milestone 3](../spec.md) (stories 12, 13, 19–22) · Plan: [J. `surt status`](../../../docs/architecture/J-cli.md#surt-status), [J. Foreground progress](../../../docs/architecture/J-cli.md#foreground-progress), [J. `--json` documents](../../../docs/architecture/J-cli.md#--json-documents)

**What to build:** a developer checking on an overnight run can tell that it is waiting out a rate limit and until when, without guessing whether it is hung. They can do this from the driving terminal, from another terminal, or from a CI log. `surt status <feature>` shows a paused line under a `RUNNING` headline. `status --json` carries the pause. The TTY footer and the 5-minute line without a TTY mention it. `surt inspect` shows when launches paused and resumed.

**Blocked by:** 02 (A Transient Failure pauses agent launches)

**Status:** ready-for-agent

- [ ] The pause is derived on every read: a run with a live Driver whose latest ended agent Execution failed `transient`, with no agent Execution started after it, is paused; nothing new is stored
- [ ] `status <feature>` keeps the `RUNNING` headline and adds one line naming the error code and the reset time from `runtime_info.resets_at`, or `retrying with backoff` when there is none
- [ ] `status --json` adds a `paused` object (`code`, plus `resets_at` when known) to the run document, additively; J's document section lists it
- [ ] The activity snapshot carries the pause; the TTY footer shows `paused until HH:MM` (or the backoff) with the waiting count, and the 5-minute non-TTY line mentions the pause
- [ ] `inspect <feature>` renders `agents.paused` and `agents.resumed` with their details; `inspect <execution>` on a rate-limited Execution shows class `transient`, the code and the reset time
- [ ] ASCII mode renders `[paused]` and passes the ASCII-only test
- [ ] Pure render tests in the `surt` crate for the status line, the footer and the inspect lines
- [ ] Binary test: while a Fake Driver is paused, `surt status` and `surt status --json` from a second process show the pause, and reads still write nothing
- [ ] J records the paused status line
