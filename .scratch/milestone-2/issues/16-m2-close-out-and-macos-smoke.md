# 16: Milestone 2 close-out and the macOS smoke check

Spec: [Milestone 2](../spec.md) (story 84, "Done when") · Slice: all · Plan: [K. After Milestone 1](../../../docs/architecture/K-roadmap.md#after-milestone-1)

**What to build:** Milestone 2 is shipped, recorded in the plan, and checked once by hand with the real Claude Code on macOS.

**Blocked by:** 05 (`surt inspect <execution-id>`), 07 (`surt logs -f`), 10 (`surt stop`), 14 (Activity footer); in practice every other M2 ticket

**Status:** ready-for-agent

- [ ] K marks M2 shipped and removes the "replacements for cut commands" notes
- [ ] Every plan update the spec lists is present (J, D, H, I. Cleanup, B's rule exception); any still missing is written
- [ ] Glyphs added by later tickets (logs, stop) go through the symbol table and pass the ASCII-only test
- [ ] CI is green on macOS, Linux and Windows with the line-coverage gate above 90%
- [ ] Manual check on macOS appended to `docs/smoke-claude-code.md`: the footer redraws during `surt start`; `surt logs -f` renders a live Builder; `surt stop` from a second terminal leaves a resumable run; `surt inspect <execution>` shows session, model, turns and cost
