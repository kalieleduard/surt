# 10: Milestone 4 close-out and the macOS smoke check

Spec: [Milestone 4](../spec.md) (stories 72–75, "Shipping") · Plan: [K. After Milestone 1](../../../docs/architecture/K-roadmap.md#after-milestone-1)

**What to build:** Milestone 4 is shipped, recorded in the plan as the end of v1, and checked once by hand with the real Claude Code on macOS.

**Blocked by:** 01 (Local `agent.toml` overrides), 02 (Local instructions and schemas), 03 (Overrides that would break a run), 04 (Fast-forward, not checked out), 05 (Fast-forward, checked out), 06 (`--merge`/`--no-merge`), 07 (Merge result in `status`), 08 (Keep a successful run), 09 (`--new-run` on a kept Subject)

**Status:** ready-for-agent

- [ ] K marks M4 implemented (the M4 rows of "Mechanisms deferred" and the M4 row of "After Milestone 1") and states that v1 is complete
- [ ] Every plan update the spec lists is present (F, A, E, I, J, D, B), and any still missing is written
- [ ] `CONTEXT.md` gains **Agent Override** (_Avoid_: custom agent, plugin), and **Cleanup** says a `succeeded` run may be kept
- [ ] No code path, message or test still says "not supported yet" for these three mechanisms
- [ ] Glyphs added by M4 go through the symbol table and pass the ASCII-only test
- [ ] An M4 section appended to `docs/smoke-claude-code.md`: a Builder override to another model, confirmed with `surt inspect <execution>`; `surt feature … --merge` on a toy repository whose `main` is checked out and clean, ending with `main` at the delivered tip; a second Feature with `on_success = "keep"` whose worktrees remain after `succeeded` and are gone after `surt delete`
- [ ] CI is green on macOS, Linux and Windows
- [ ] The manual check on macOS passes and is recorded under "Smoke results"

## Smoke results

One line per run of the Milestone 4 section of [the smoke checklist](../../../docs/smoke-claude-code.md):
