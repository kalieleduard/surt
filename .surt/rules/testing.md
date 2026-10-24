<testing_rules>
  <scope>
    Apply to changed behavior and tests. Follow clean-code.md, including the comment ban. Avoid unrelated refactors.
  </scope>

  <principles>
    - Test observable behavior and contracts, not implementation details.
    - Test real scenarios, boundaries, failures, or known bugs. Ban trivial, duplicate, assertion-free, and coverage-only tests.
    - Use descriptive names that state the expected behavior, and assert outcomes and side effects.
    - Write the production logic for every valid input; never special-case values a test happens to use.
  </principles>

  <unit_tests>
    - Keep tests fast, isolated, deterministic, and focused.
    - Cover success, invalid input, boundaries, errors, and transitions.
    - Mock external boundaries, not domain logic. Avoid testing private methods.
  </unit_tests>

  <integration_tests>
    - Test component, adapter, and storage interactions, including transactions and recovery where relevant.
    - Use temporary databases, repositories and directories (tempfile), and reset fixtures between tests.
    - Use the Fake Runtime instead of launching a real agent.
    - Avoid network, wall-clock timing, shared-state, or execution-order dependencies in CI.
  </integration_tests>

  <regression>
    - For each bug fix, reproduce the failure and assert the corrected behavior.
    - Confirm red before green when feasible; retain the test.
  </regression>

  <coverage>
    <why>CI fails a build whose workspace line coverage is not above 90%, so a change that drops it cannot merge.</why>
    - Require project-wide line coverage above 90%. Never lower this threshold.
    - Measure with: cargo llvm-cov --workspace --features test-support --summary-only
    - Review branch coverage where available; prioritize meaningful gaps.
    - Never exclude production code, skip failures, mark tests ignored, or weaken assertions to raise coverage or make a check pass. Justify tool exclusions.
    - Report coverage below threshold or unmeasured; do not claim compliance.
  </coverage>

  <verification>
    Run cargo test --features test-support and, when cargo-llvm-cov is installed, the coverage command above. In the Checks line of the clean-code.md report, give commands, PASS/FAIL/NOT RUN, coverage %, and blockers. Report only results you saw.
  </verification>
</testing_rules>
