# A Conversation continues across a Loop

An Agent whose profile says `conversation = "loop"` continues the Claude conversation of its last successful Execution in the same Loop, across Iterations and Attempts, instead of starting a fresh session. Each Execution still gets its own session id, forked from the previous one, and receives only the Inputs that changed. When the previous conversation cannot be continued, the Execution restarts fresh with every Input, so correctness never depends on the transcript. The feature Template's Spec Agent uses it to ask the developer questions through an `input` step until it delivers a spec with none left open.

## Considered Options

- **A fresh session per Execution (v1):** simplest and fully reconstructible, but every round of questions rereads the repository and loses why the agent asked. A spec that needs three rounds is three strangers reading each other's notes.
- **A live process waiting for the answer (`--input-format stream-json`):** keeps the whole conversation in memory, but the Driver must stay up while the developer thinks, and a Ctrl-C, a reboot or a `kill -9` loses everything. Recovery could not resume it days later.
- **An interactive `claude` in the developer's terminal:** the most natural chat, but there is no structured delivery to validate, and Interruption and the delivery contract of F no longer apply.
- **Continue a forked session per Execution, with a fresh restart as fallback (chosen):** the agent keeps its memory between rounds, each Execution keeps one session id and one log, an interrupted round never pollutes the chain, and losing the transcript costs memory, not correctness.

## Consequences

- The brief's "a new Claude session per execution" is narrowed to "a new session id per Execution". F's ban on `--resume` and `--fork-session` is lifted for continued Executions only, and session persistence is on for `"loop"` Agents.
- Answers accumulate in one artifact, so a restart needs only the latest Revisions. Recovery still decides everything from rows, Revisions and Git.
- `runtime_info` records `continued_from` or `restarted` with a reason, so a developer can see when the agent lost its memory.
- Claude Code's transcripts become an input Surt reads but does not own: their location, retention (`cleanupPeriodDays`) and format can change with a Claude Code upgrade, and the only effect is a restart.
- The conversation never crosses a step, a Loop or a run. The Task Agent, Builders and Reviewers keep starting with no memory.
