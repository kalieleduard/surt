<testing_rules>
  <scope>
    Apply to changed behavior and tests. Follow clean-code.md. Never add code comments or unrelated refactors.
  </scope>

  <principles>
    - Test observable behavior and contracts, not implementation details.
    - Test real scenarios, boundaries, failures, or known bugs. Ban trivial, duplicate, assertion-free, and coverage-only tests.
    - Use descriptive names and assert outcomes and side effects.
  </principles>

  <unit_tests>
    - Keep tests fast, isolated, deterministic, and focused.
    - Cover success, invalid input, boundaries, errors, and transitions.
    - Mock external boundaries, not domain logic. Avoid testing private methods.
  </unit_tests>

  <integration_tests>
    - Test component, adapter, and storage interactions, including transactions and recovery where relevant.
    - Use temporary databases and reset fixtures between tests.
    - Avoid network, timing, shared-state, or execution-order dependencies in CI.
  </integration_tests>

  <regression>
    - For each bug fix, reproduce the failure and assert the corrected behavior.
    - Confirm red before green when feasible; retain the test.
  </regression>

  <coverage>
    - Require project-wide line coverage >90%; fail CI at or below 90%. Never lower this threshold.
    - Review branch coverage where available; prioritize meaningful gaps.
    - Never exclude production code, skip failures, or weaken assertions to inflate coverage. Justify tool exclusions.
    - Report coverage below threshold or unmeasured; do not claim compliance.
  </coverage>

  <verification>
    Run `cargo test` and the configured coverage tool for Rust. In the existing Checks line, report commands, PASS/FAIL/NOT RUN, coverage %, and blockers. Never invent results.
  </verification>
</testing_rules>