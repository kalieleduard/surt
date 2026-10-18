# 07: Milestone 3 close-out and the macOS smoke check

Spec: [Milestone 3](../spec.md) (stories 47–50, "Shipping") · Plan: [K. After Milestone 1](../../../docs/architecture/K-roadmap.md#after-milestone-1)

**What to build:** Milestone 3 is shipped, recorded in the plan, and checked once by hand with the real Claude Code on macOS.

**Blocked by:** 01 (Global config), 02 (Transient Failure pause), 03 (Pause visible), 04 (Agent Slots), 05 (Live limit), 06 (Project-wide slot count)

**Status:** ready-for-human

- [x] K marks M3 implemented: the M3 rows of "Mechanisms deferred" and the M3 row of "After Milestone 1"
- [x] Every plan update the spec lists is present (A, I, J, D, B), and any still missing is written; B's `Ctx` and `Progress` match what was built
- [x] `CONTEXT.md` gains **Agent Slot** (_Avoid_: worker, seat), and **Transient Failure** says Surt pauses every agent launch of the Driver
- [x] Glyphs added by M3 go through the symbol table and pass the ASCII-only test
- [x] An M3 section appended to `docs/smoke-claude-code.md`: two Features in one toy project driven at once with `max_concurrent_agents = 1` never run two Builders (checked with `surt status` from a third terminal); `stop_grace_period = "30s"` in the global file, and a `surt stop` that lets a Builder exit
- [ ] CI is green on macOS, Linux and Windows
- [ ] The manual check on macOS passes and is recorded under "Smoke results"

## Smoke results

One line per run of the Milestone 3 section of [the smoke checklist](../../../docs/smoke-claude-code.md#milestone-3-run-unattended-with-the-real-claude-code):

## Comments

- 2026-10-18 (implementer): K marks M3 implemented (the three Mechanisms rows were already marked by earlier tickets; the "After Milestone 1" row now reads implemented, shipped once CI and the macOS smoke pass, and M4 is next). The plan audit found A, I, J, D, G and B's removed slot exception already written by tickets 01–06; B's `Ctx` sketch still showed `free_slots: usize` and `Timestamp`, and now matches the code (`free_slots: Vec<usize>`, `paused_until: Option<i64>`, `now: i64`, `Launch { step, slot }`, `decision` returning `Decision { actions, queued }`). `CONTEXT.md` gains **Agent Slot**; **Transient Failure** already named the pause. Every M3 glyph (⏸, ↻, ⚠, the footer pause) is in the symbol table; new ASCII-mode tests cover the paused heartbeat line, the `agents.paused`/`agents.resumed` timeline and a rate-limited Execution's detail. Fixed a stray backspace character in `ActivityPause`'s doc comment. `docs/smoke-claude-code.md` no longer calls the grace period fixed or a rate limit blocking, and gains the M3 section. Open: CI on the three OSes (needs a push) and the manual macOS check.
- 2026-10-18 (orchestrator): Merged into `integration/milestone-3` at 23a88e5; code-review fixes at 5e84027. Open: CI on macOS, Linux and Windows (needs the branch pushed) and the manual macOS smoke (M3 section of `docs/smoke-claude-code.md`).
