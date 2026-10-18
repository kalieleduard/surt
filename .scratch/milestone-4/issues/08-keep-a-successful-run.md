# 08: `on_success = "keep"` keeps a successful run

Spec: [Milestone 4](../spec.md) (stories 61–68, 71) · Plan: [I. Cleanup — what each ending removes](../../../docs/architecture/I-cleanup.md#what-each-ending-removes), [E. File layout](../../../docs/architecture/E-workflow-schema.md#file-layout), [J. Configuration](../../../docs/architecture/J-cli.md#configuration)

**What to build:** a developer tuning their Agents sets `[cleanup] on_success = "keep"` in `.surt/config.toml`, and a successful run leaves its worktrees, Task Branches, Salvage Refs, Working Copy and Execution directories for them to inspect until `surt delete`. A kept run never looks like a failed or pending cleanup, and cancelled runs clean as before.

```
✔ run        succeeded
· cleanup    kept (on_success = "keep") — remove with: surt delete 20261016-fix-cache
```

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] `.surt/config.toml` accepts `[cleanup] on_success = "clean" | "keep"`, absent by default; any other value refuses every command with the file, the line and the reason (exit `3`)
- [ ] Workflow Definitions' `[cleanup] on_success` also parses `"keep"`, and its canonical form keeps it
- [ ] The effective policy is the configuration's value when set, else the root Workflow Definition's; child definitions are not consulted; it is frozen into the `config` snapshot
- [ ] A `succeeded` run with `keep` runs no cleanup step, leaves `cleaned_at` NULL, writes no cleanup event and prints the kept line; status derives `CleanupState::Kept` from the frozen policy
- [ ] No command treats a kept run as cleanup pending; driving it again removes nothing; `surt delete` removes it as I's table says
- [ ] A cancelled run ignores the policy
- [ ] Config unit tests: `clean`, `keep`, absent, invalid
- [ ] `Project` tests with the Fake and real Git: a kept run leaves every item of I's `keep` column and status reports `Kept`; driving it again removes nothing; `delete` removes it; a cancelled run with `keep` cleans as cancelled
- [ ] Binary tests: the kept line; `kept` in `status`, `status --all` and `cleanup.state = "kept"` in `status --json`
- [ ] E, I and J record the configuration key and the effective policy; D lists `on_success` in the `config` snapshot; K's `on_success = "keep"` row says it is implemented
