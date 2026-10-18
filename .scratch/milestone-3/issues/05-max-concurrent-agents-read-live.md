# 05: `max_concurrent_agents` read live

Spec: [Milestone 3](../spec.md) (stories 29–32) · Plan: [A. Agent concurrency limit](../../../docs/architecture/A-process-model.md#agent-concurrency-limit), [J. Configuration](../../../docs/architecture/J-cli.md#configuration)

**What to build:** a developer can throttle or speed up a long run without stopping it. Every running Driver reads `max_concurrent_agents` from `.surt/config.toml` before each tick. A lowered limit stops offering the slots at or above it, and agents already holding those slots finish. A raised limit is used on the next tick. If the developer saves an invalid file while a Driver runs, the Driver keeps its last valid limit and warns once instead of crashing the run.

**Blocked by:** 04 (Agent Slots as project-wide lock files)

**Status:** ready-for-agent

- [x] The Driver rereads the project configuration before each tick; only `max_concurrent_agents` is applied live, and the frozen keys of the `config` snapshot stay frozen
- [x] Lowering the limit never stops a running agent; raising it offers the new slot indices on the next tick
- [x] A parse failure mid-run keeps the last valid limit and emits one warning progress event, which repeats only after the file has parsed again and then broken again
- [x] The CLI prints the warning through the symbol table
- [x] `Project::drive` tests: lowering the limit from 2 to 1 mid-run lets the running agents finish and then launches one at a time; raising it from 1 to 2 launches a second agent without restarting; an invalid file mid-run keeps the limit and warns once
- [x] A records the live-limit warning
