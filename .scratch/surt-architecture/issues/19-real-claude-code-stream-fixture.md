# Capture a real Claude Code stream as a fixture

Map: [Surt — Architecture and implementation plan](../map.md)
Type: task
Status: resolved
Assignee: Kaliel

## Question

What do the `stream-json` events of a successful Claude Code run, and of the failures the runtime needs to classify, really look like?

The research in [Claude Code CLI headless capabilities](02-claude-code-cli-headless.md) did not observe a real run (expired OAuth). Checklist (HITL — requires a valid login):
1. `claude auth status` / re-login.
2. In a temporary directory, run with the flag combination from [F. Agent Contract](../../../docs/architecture/F-agent-contract.md#claude-code-launcher-and-compatibility) (`dontAsk`, a new `--session-id`, `--json-schema` of the `outcome`/`outputs` envelope) and save stdout.
3. Repeat with: an invalid model, `--max-turns 1` on a task that needs more, SIGTERM midway.
4. Note the exit codes and save the streams in `docs/research/fixtures/claude-code/` on the `research/claude-code-cli-headless` branch.

Check in the success stream: `system/init` (model, permission mode, tools), `result` with `structured_output`, and whether there are events after the `result`.

The result feeds the Claude Code Runtime's parser and the Fake Runtime's scripts. Rate limits and network outages are left out: an unknown category already falls into `retryable` ([I](../../../docs/architecture/I-recovery.md#transient-failures-network-and-rate-limits)).

## Answer

Done on 2026-10-17 with Claude Code 2.1.301 (macOS, subscription login, `claude-haiku-5-5`, `--effort low`) and the flags from [F](../../../docs/architecture/F-agent-contract.md#claude-code-launcher-and-compatibility). Sanitized fixtures, `capture.sh` and a findings table in `docs/research/fixtures/claude-code/` on the `research/claude-code-cli-headless` branch (commit `378e0b3`, local only).

Seven scenarios: `success`, `bad-model`, `auth-expired`, `max-turns`, `sigterm`, `schema-miss` (the model refuses the structured output) and `permission-denied` (a `Write(...)` rule).

| Scenario | Exit | `result` | Files left behind |
|---|---|---|---|
| success | 0 | `success`, `is_error=false`, valid `structured_output` | `note.md` |
| invalid model / expired auth | 1 | `subtype=success` **with** `is_error=true`, `terminal_reason=api_error`; `assistant.error` = `model_not_found` / `authentication_failed` | none |
| `--max-turns 1` | 1 | `error_max_turns` | all (5 parallel Writes in one turn) |
| SIGTERM | 143 | **no** `result`; the stream ends on a `tool_result` | partial |
| refusal / denied write | 0 | `success` with a valid envelope and prose in place of the path | none |

What changed in the plan (already written in [F](../../../docs/architecture/F-agent-contract.md#runtime-success-and-failure) and [I](../../../docs/architecture/I-recovery.md#transient-failures-network-and-rate-limits)):

- The launcher is validated end to end (subscription, macOS). `--bare` and Windows still have no real run; they are left to CI and the manual smoke test of [16](16-test-strategy.md).
- The child's environment by **allowlist**, not by removal: `CLAUDE_CODE_SIMPLE`/`ANTHROPIC_BASE_URL` inherited from a Claude Code session hid a valid login.
- `Write(path)` is silently ignored; only `Edit(//abs/**)` allows writing (F already said so, now with evidence).
- `--json-schema` forces an envelope even when the work failed: a valid envelope does not prove delivery; the file check (step 5 of F) is the real gate.
- `init` check: the requested model is echoed even if it does not exist; `plugins` stays populated in `--safe-mode` — check `tools`/`mcp_servers`; `system/ui_invalidate` may come before `init`.
- `rate_limit_event` arrives in every run with `status: "allowed"`; only another status is a limit. A real rate limit and a network outage were not captured (they stay `retryable` by default, as in I).

No new product decision, ADR or `CONTEXT.md` term.
