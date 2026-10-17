# Claude Code CLI headless capabilities

Map: [Surt — Architecture and implementation plan](../map.md)
Type: research
Status: resolved

## Question

What does the installed Claude Code CLI actually offer for non-interactive execution controlled by an orchestrator?

Survey from primary sources (official docs, `claude --help`, changelog): model flags and aliases (`--model`), how **effort** is set (flag, setting, env?), permission modes and tool allow/deny, `--output-format stream-json` (event schema, final event with cost/usage/`session_id`), `--session-id`/guaranteed new-session generation, `--append-system-prompt`/`--system-prompt`, structured output validated by JSON Schema (if it exists), exit codes and signals, behavior on SIGTERM/SIGINT (and the Windows equivalent), working directory/`--add-dir`, how to detect the version and validate models/aliases before running, Windows support, relevant environment variables, settings/hooks that leak in from the user (global CLAUDE.md, hooks) and how to isolate them.

Findings: `docs/research/claude-code-cli-headless.md` on the `research/claude-code-cli-headless` branch.

## Answer

Full findings: `docs/research/claude-code-cli-headless.md` on the `research/claude-code-cli-headless` branch (commit `172db83`; Claude Code v2.1.300, macOS). **Caveat:** the local OAuth had expired, so no run reached the model — the success event formats and `structured_output` come from the documentation only.

Facts that constrain the design (not decisions):
- Every needed control has a flag: `--model`, `--effort`, `--permission-mode`/`--tools`/`--allowedTools`/`--disallowedTools`, `--output-format stream-json --verbose` (without `--verbose` it exits with 1), `--json-schema`, `--session-id` (UUID; refuses an existing id), `--no-session-persistence`, `--system-prompt[-file]`/`--append-system-prompt[-file]`, `--max-turns`, `--max-budget-usd`, isolation via `--bare`/`--settings`/`--setting-sources`/`--strict-mcp-config`.
- Success requires combining exit code + `is_error` + `subtype` + presence of `structured_output`; `result` may not be the last line; the parser must tolerate unknown types/fields.
- There is no way to validate model/effort through the CLI before launch; an invalid effort is ignored with a warning; aliases vary by version/provider — pin full IDs; the resolved model shows up in `system/init`.
- The default permission mode depends on hidden state — always pass an explicit `--permission-mode`; `bypassPermissions` is refused as root.
- Default `-p` is **not isolated** (the user's hooks, MCP, plugins and CLAUDE.md load); `--bare` isolates but requires `ANTHROPIC_API_KEY`/`apiKeyHelper` (it does not read the subscription login); a custom `CLAUDE_CONFIG_DIR` also moves the credentials.
- SIGTERM → exit 143, kills Bash trees, no turn result; **there is no wall-clock timeout** (Surt must impose its own); a run can stay open for up to ~10 min waiting on background work. Stop semantics on Windows are undocumented.
- Near-daily releases; `claude --version` works without auth; `system/init` exposes the version and `capabilities`; auto-update can swap the binary between runs (can be disabled via env).

Open questions carried to other tickets: auth posture × isolation and timeouts (Agent contract), stop semantics on Windows (Cross-platform primitives / Process model), real stream fixture (new ticket 19).
