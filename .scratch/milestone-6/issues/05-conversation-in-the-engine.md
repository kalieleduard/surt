# 05: A conversation continues across a Loop (engine and Fake)

Spec: [Milestone 6](../spec.md) (Conversation) · Plan: [F](../../../docs/architecture/F-agent-contract.md), [I. Recovery](../../../docs/architecture/I-recovery.md), [ADR 0014](../../../docs/adr/0014-a-conversation-continues-across-a-loop.md)

**What to build:** an Agent with `conversation = "loop"` is asked to continue its last successful Execution in the same Loop, with a delta context; the Fake honours it and can simulate a lost transcript.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] `[profile] conversation = "none" | "loop"` (default `"none"`), in the canonical `agent.toml`; a `"loop"` Agent used by a step outside every Loop body refuses at creation
- [x] The Driver finds the latest `succeeded` Execution with a `session_id` of the same step key in the same Loop of the same run (any earlier Iteration or Attempt); interrupted, cancelled and failed ones are never chosen
- [x] `RuntimeRequest.continuation: Option<Continuation { session_id, execution_id, context }>`; the Context Assembler also writes `context-continued.md` with only the Inputs whose Revision differs from the previous Execution's `execution_input`, excluding Revisions that Execution produced
- [x] `RuntimeReport` carries the conversation result; `runtime_info.conversation` is `{continued_from}`, `{restarted, previous}` or `{fresh: "no_previous"}` for `"loop"` Agents and absent otherwise; `inspect <execution>` prints it
- [x] Fake runtime: reports `continued_from` when asked to continue; a scripted `transcript_missing` behaviour (Rust and TOML scripts) restarts with the full context in the same Execution and reports `restarted`
- [x] Engine tests: the second Execution continues the first; failed/interrupted ones are skipped; a rejection continues; the delta includes a human edit and omits the Agent's own output; `transcript_missing` restarts and spends no Attempt; `"none"` Agents have no continuation
- [x] F and I document the mechanism
