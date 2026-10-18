# 07: Milestone 3 close-out and the macOS smoke check

Spec: [Milestone 3](../spec.md) (stories 47–50, "Shipping") · Plan: [K. After Milestone 1](../../../docs/architecture/K-roadmap.md#after-milestone-1)

**What to build:** Milestone 3 is shipped, recorded in the plan, and checked once by hand with the real Claude Code on macOS.

**Blocked by:** 01 (Global config), 02 (Transient Failure pause), 03 (Pause visible), 04 (Agent Slots), 05 (Live limit), 06 (Project-wide slot count)

**Status:** ready-for-agent

- [ ] K marks M3 implemented: the M3 rows of "Mechanisms deferred" and the M3 row of "After Milestone 1"
- [ ] Every plan update the spec lists is present (A, I, J, D, B), and any still missing is written; B's `Ctx` and `Progress` match what was built
- [ ] `CONTEXT.md` gains **Agent Slot** (_Avoid_: worker, seat), and **Transient Failure** says Surt pauses every agent launch of the Driver
- [ ] Glyphs added by M3 go through the symbol table and pass the ASCII-only test
- [ ] An M3 section appended to `docs/smoke-claude-code.md`: two Features in one toy project driven at once with `max_concurrent_agents = 1` never run two Builders (checked with `surt status` from a third terminal); `stop_grace_period = "30s"` in the global file, and a `surt stop` that lets a Builder exit
- [ ] CI is green on macOS, Linux and Windows
- [ ] The manual check on macOS passes and is recorded under "Smoke results"

## Smoke results

One line per run of the Milestone 3 section of [the smoke checklist](../../../docs/smoke-claude-code.md):
