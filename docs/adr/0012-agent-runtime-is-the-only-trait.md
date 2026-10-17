# The Agent Runtime is the only trait; store, Git and clock are not abstracted

`AgentRuntime` is the only trait at a seam in Surt, with two adapters: the Claude Code runtime and the Fake Runtime. The store (SQLite), Git (the `git` CLI), the six Operations and the clock have no trait. Tests run them for real: a SQLite file and a Git repository in a temporary directory, and Tokio's paused time for timers.

## Considered Options

- **Ports for everything (`StateStore`, `GitOps`, `Clock`, `Operation`):** every test could swap in an in-memory double. But each port would have one production adapter and one fake that must imitate SQLite's transactions or Git's ref semantics. Recovery bugs live exactly in those semantics, so the fakes would hide the bugs the tests exist to find.
- **No trait at all, an `enum` of two runtimes:** possible because the set is closed today, but the Fake would then live inside the production dispatch rather than at a seam tests own.
- **One trait where two real adapters exist (chosen):** Claude Code costs tokens and needs an account, so it is the one dependency a test cannot run locally. SQLite and Git are local, fast and deterministic. Timers are covered by `tokio::time::pause()`, and computations that need the wall clock take `now` as an argument.

## Consequences

- The trait uses native `async fn` in traits with a `Send` future and is not object-safe; `drive` takes `&impl AgentRuntime`, so the facade carries no type parameter.
- Engine tests need `git` installed and touch the filesystem. They are slower than pure in-memory tests, and they exercise the code that ships.
- A second real AI runtime (out of scope for v1) would be a new adapter of the same trait, not a new seam.
- This is a deliberate exception to the dependency-inversion rule of [`rules/architecture.md`](../../rules/architecture.md), whose own example asks for a Storage port. It is reported in the rules' format:
  ```
  Exceptions: DIP; surt-engine drive:Store | DIP; surt-engine drive|reconcile|operations:git; one adapter each, and a fake would hide the transaction and ref semantics recovery depends on; drive cannot run without SQLite and git, so its tests are slower and need a temp directory
  ```
  What the rule protects still holds: the decision logic (`decide()`, `RunGraph`, `Action`, the states of section G) is plain Rust that imports neither `sqlx` nor the `git` module, and is tested without I/O. The store maps rows into `RunGraph`; it never hands SQL types to `drive` ([B](../architecture/B-workspace.md#the-scheduler-is-a-pure-function)). A Storage port is added the day a second adapter exists.
