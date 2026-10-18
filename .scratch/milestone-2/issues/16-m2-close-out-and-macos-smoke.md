# 16: Milestone 2 close-out and the macOS smoke check

Spec: [Milestone 2](../spec.md) (story 84, "Done when") · Slice: all · Plan: [K. After Milestone 1](../../../docs/architecture/K-roadmap.md#after-milestone-1)

**What to build:** Milestone 2 is shipped, recorded in the plan, and checked once by hand with the real Claude Code on macOS.

**Blocked by:** 05 (`surt inspect <execution-id>`), 07 (`surt logs -f`), 10 (`surt stop`), 14 (Activity footer); in practice every other M2 ticket

**Status:** ready-for-human

- [x] K marks M2 shipped and removes the "replacements for cut commands" notes
- [x] Every plan update the spec lists is present (J, D, H, I. Cleanup, B's rule exception); any still missing is written
- [x] Glyphs added by later tickets (logs, stop) go through the symbol table and pass the ASCII-only test
- [ ] CI is green on macOS, Linux and Windows with the line-coverage gate above 90%
- [ ] Manual check on macOS appended to `docs/smoke-claude-code.md`: the footer redraws during `surt start`; `surt logs -f` renders a live Builder; `surt stop` from a second terminal leaves a resumable run; `surt inspect <execution>` shows session, model, turns and cost

## Smoke results

One line per run of [the Milestone 2 checklist](../../../docs/smoke-claude-code.md#milestone-2-operate-and-debug-a-run-with-the-real-claude-code):

## Comments

- 2026-10-18 (implementer): K marks M2 shipped. The plan audit found J, D, H, I. Cleanup and B's exception already written by tickets 02–15; three "decided here" points were missing and are now in J (`--new-run` finishes the previous run's pending cleanup first, restore's in-house line counts, the engine giving log directory paths). ASCII-mode tests now cover logs, stop, restore, delete, inspect (timeline and Execution) and the stop progress lines. Local line coverage (`cargo llvm-cov --workspace --features test-support`, as CI): 95.57% before this ticket's tests. Open: CI on the three OSes (needs a push) and the manual macOS check (M2 section of `docs/smoke-claude-code.md`).
- 2026-10-18 (orchestrator): Implemented and green on macOS. Merged into `integration/milestone-2` at 7e82ee2; code-review fixes at 2c96dfe. Open criteria: CI green on macOS, Linux and Windows (needs the branch pushed) and the manual macOS smoke with the real Claude Code (M2 section of `docs/smoke-claude-code.md`); record it under "Smoke results".
