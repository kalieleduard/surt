# 01: Delivery checks are declared on Agent outputs and frozen

Spec: [Milestone 5](../spec.md) (stories 18–20) · Plan: [F. Agent Contract](../../../docs/architecture/F-agent-contract.md), [ADR 0013](../../../docs/adr/0013-definitions-live-in-the-project.md)

**What to build:** the three delivery checks stop being registered by the binary with `Catalog::delivery_check(agent, output, check)` and become a field of the output in `agent.toml`. They are frozen with the Agent, so `launch.rs` reads them from the run's snapshot, not from the live catalog.

```toml
[outputs.feedback]
format = "json"
json_schema = "schemas/feedback.schema.json"
checks = [{ kind = "reviewed_commit" }, { kind = "no_blocking_finding", outcome = "approved" }]
```

**Blocked by:** None (can start immediately)

**Status:** open

- [ ] `OutputSpec` gains optional `checks`: an array of tables `{ kind = "commit_evidence" | "reviewed_commit" | "no_blocking_finding", outcome = "<outcome>" }`; `outcome` is required for `no_blocking_finding` and refused for the others; unknown kinds and fields refused (`deny_unknown_fields`)
- [ ] The static checks of `closure.rs`'s `check_deliveries` (output is `format = "json"`, outcome declared by the Agent) move into `AgentContract::check`, keeping their problem kinds and messages
- [ ] Checks are serialized in the canonical `agent.toml`, so they are part of the Agent snapshot identity; `drive/launch.rs` takes them from the frozen Agent
- [ ] `Catalog::delivery_check`, the `checks` map and the `StepUse::DeliveryCheck` blame path are removed; the M4 rule "an override that drops a checked output is refused" becomes "the declared checks validate"
- [ ] `crates/surt/assets/agents/builder/agent.toml` declares `commit_evidence` on `commit`; the Reviewer declares `reviewed_commit` and `no_blocking_finding` (`approved`) on `feedback`; `builtin.rs` stops registering checks; `builtin/tests.rs` asserts the declared checks
- [ ] `Project` tests with the Fake: declared checks are enforced as before; editing the Agent's checks in `.surt/agents/` after creation does not change the run's; an invalid declaration refuses `create_run` with no row written
- [ ] F documents `checks` on outputs

## Comments
