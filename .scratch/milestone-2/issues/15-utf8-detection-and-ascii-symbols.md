# 15: UTF-8 console detection and ASCII symbols

Spec: [Milestone 2](../spec.md) (stories 75, 76, 83) · Slice: R8 Terminal · Plan: [J. Output channels and non-interactive use](../../../docs/architecture/J-cli.md#output-channels-and-non-interactive-use)

**What to build:** on an old Windows console or a non-UTF-8 locale, output stays readable. The symbols become `[ok] [error] [warn] [retry] [paused] [blocked]`, and every other non-ASCII glyph the CLI prints gets an ASCII equivalent. `NO_COLOR` keeps disabling colour independently.

Prefactor first: every glyph the CLI prints today (spread over messages, status, show, init, decide, refusals and progress) goes through one symbol table, so later commands add glyphs in one place.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [x] Every glyph is printed through one symbol table in the `surt` crate
- [x] The engine's `platform` module answers "does the console output UTF-8" (Windows: output code page ≠ 65001; Unix: the first set of `LC_ALL`, `LC_CTYPE`, `LANG` doesn't name UTF-8), re-exported through the facade
- [x] ASCII mode maps `✔ ✖ ⚠` and the retry/paused/blocked symbols to J's six, and `● ⏳ ⏹ ✎ ⟳ │ ─ … −` to ASCII equivalents
- [x] `NO_COLOR` and the symbol set are independent
- [x] Pure test: every message the CLI can print contains only ASCII bytes in ASCII mode
- [x] `platform` unit test runs on each OS in CI
