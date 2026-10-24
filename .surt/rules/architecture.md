<architecture>
  <scope>
    - Apply to changed modules, direct dependencies, and tests.
    - Preserve behavior; ignore unrelated legacy code.
    - Exclude generated/vendor code and historical migrations unless requested.
  </scope>

  <priorities>
    Correctness/contracts > dependency direction > cohesion > patterns.
    Report justified exceptions.
  </priorities>

  <boundaries>
    <why>The engine must stay testable with the Fake Runtime and temporary repositories, so policy cannot depend on concrete processes, files or databases.</why>
    - Keep domain logic independent of frameworks, databases, CLI, HTTP, and APIs.
    - Use cases coordinate behavior and define ports for external dependencies.
    - Adapters implement ports; wire at the composition root.
    - Dependencies point inward; avoid cycles and cross-layer shortcuts.
    - The surt crate (CLI) depends on surt-engine, never the reverse.
  </boundaries>

  <solid>
    - SRP: one reason to change.
    - OCP: extend only for real variation.
    - LSP: implementations preserve their contracts.
    - ISP: small, consumer-focused interfaces/traits.
    - DIP: high-level policy depends on ports, not concrete infrastructure.
    - Inject through constructors/functions; avoid service locators and mutable globals.
  </solid>

  <simplicity>
    - Add ports only for real boundaries or substitutions.
    - Avoid interfaces per type, repositories per table, and speculative layers.
    - Keep cohesive features together; follow project idioms.
    - Example: inject a Storage port into a use case instead of a SQLite connection.
  </simplicity>

  <verification>
    - Test domain without I/O; test adapters at boundaries.
    - Report with the three-line format of clean-code.md.
    - Example exception: DIP; src/legacy.rs:load; fixed vendor API; adapter extraction deferred.
  </verification>
</architecture>
