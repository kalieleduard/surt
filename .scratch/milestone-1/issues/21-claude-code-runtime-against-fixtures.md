# 21: Claude Code runtime against fixtures

Spec: [Milestone 1](../spec.md) (stories 105–109) · Slice: S6 Claude Code (parallel from S3) · Plan: [F. Claude Code launcher](../../../docs/architecture/F-agent-contract.md#claude-code-launcher-and-compatibility), [F. Runtime success and failure](../../../docs/architecture/F-agent-contract.md#runtime-success-and-failure), [ticket 19](../../surt-architecture/issues/19-real-claude-code-stream-fixture.md)

**What to build:** a `ClaudeCodeRuntime` that implements `AgentRuntime` for the real Claude Code on the developer's subscription login: it assembles an isolated launch, parses the stream-json output and only accepts an Execution that meets every success criterion, classifying failures so retries are spent wisely. Proven against real captured streams; the real Claude Code never runs in CI.

**Blocked by:** 07 (AgentRuntime trait, Fake Runtime and delivery validation)

**Status:** ready-for-agent

- [x] The argv follows F: `-p`, a new `--session-id`, `--no-session-persistence`, `--output-format stream-json --verbose`, full model and effort, `--permission-mode dontAsk`, explicit per-role tools and rules, a generated `--json-schema`, `--append-system-prompt-file`, a generated `--settings`, strict empty MCP, `--disable-slash-commands`, `--safe-mode --setting-sources user`; never `--resume`/`--continue`; the context goes through stdin, which is then closed
- [x] The child environment is built from an allowlist plus the variables that disable auto-update, memory and CLAUDE.md loading; variables like gateway URLs, model overrides and API keys never pass through
- [x] Write path rules use the `Edit(...)` namespace with `//` for an absolute POSIX root
- [x] The parser reads until the process exits and the stream drains, ignores unknown types and fields, and does not assume `system/init` comes first
- [x] Acceptance requires exit 0, initialization matching session, model, version, permission mode and tools, a `success` result with `is_error = false` and structured output, and a valid envelope; `subtype = "success"` alone is not enough
- [x] Classification reads `assistant.error` and `terminal_reason`: auth, missing or mismatched model and missing executable are `not_retryable`; rate limit, overload and network are `transient` with the reset time when reported; unknown is `retryable`
- [x] Fixtures (success, auth failure, nonexistent model, SIGTERM 143 without `result`) are copied from branch `research/claude-code-cli-headless`; tests cover the parser, acceptance, classification, argv and environment assembly
