# 06: Project-wide slot count in `status`

Spec: [Milestone 3](../spec.md) (stories 33, 34) · Plan: [J. `surt status`](../../../docs/architecture/J-cli.md#surt-status), [J. Reading never writes](../../../docs/architecture/J-cli.md#reading-never-writes)

**What to build:** a developer running two Features at once sees true slot numbers. `surt status` prints `agent slots: N/M in use` counted across every live Driver of the project. A queued Task's `waiting for agent slot (N/M in use)` explains that it waits for the other Feature's agents. `status --json`'s `agent_slots` agrees. The count comes from SQLite, not from probing the lock files, so reading still never writes or briefly takes a slot.

**Blocked by:** 04 (Agent Slots as project-wide lock files)

**Status:** ready-for-agent

- [x] The count is the number of running agent Executions holding a `slot` across every run of the project whose Driver is live
- [x] The overview line, the per-Task queued detail and `status --json`'s `agent_slots` all use it; `M` is the current `max_concurrent_agents`
- [x] `status` never touches the slot lock files
- [x] `Project` test: with two Features driven at once, the status of each reports the project-wide count
- [x] Binary test: `surt status` from a third process shows `agent slots: 1/1 in use` and the other Feature's Task `waiting for agent slot (1/1 in use)`; the "reads never write" hash check passes
- [x] J records that the count is project-wide and derived from SQLite
