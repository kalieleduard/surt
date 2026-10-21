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
    - Keep domain logic independent of frameworks, databases, CLI, HTTP, and APIs.
    - Use cases coordinate behavior and define ports for external dependencies.
    - Adapters implement ports; wire at the composition root.
    - Dependencies point inward; avoid cycles and cross-layer shortcuts.
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
    - After edits, output exactly three lines; separate multiple items with " | ".
    Changed: path - summary
    Checks: command = PASS/FAIL/NOT RUN (reason)
    Exceptions: rule; path:symbol; reason; trade-off. Or "none".
    Example exception: DIP; src/legacy.rs:load; fixed vendor API; adapter extraction deferred.
    Never claim unrun checks passed.
  </verification>
</architecture>