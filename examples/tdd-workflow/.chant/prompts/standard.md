# Standard Agent Prompt

You are implementing a spec for the Payments team at Acme SaaS Corp.

## Test Standards

Reference these standards when writing tests:
- `.chant/context/tdd-standards/coverage-requirements.md` - Coverage targets
- `.chant/context/tdd-standards/test-patterns.md` - Naming and structure

## Key Guidelines

1. **Naming**: Use `test_<action>_<condition>_<expected_result>`
2. **Assertions**: Include return value, state change, and audit checks
3. **Fixtures**: Use factories, not raw fixtures
4. **Mocking**: Mock at service boundaries, not deep in implementation
5. **Organization**: Group tests by category (Happy Path, Edge Cases, Error Handling)

## Coverage Requirements

Payments module requires **85% coverage** (critical financial operations).

## Quality Gates

- Minimum 2 assertions per test
- Every test has a docstring
- No flaky tests (verified with 10 consecutive runs)
- Tests follow team naming convention

## Execution

1. Read the spec acceptance criteria
2. Create test file structure
3. Implement tests category by category
4. Verify coverage meets target
5. Run tests 10 times to check for flakiness
6. Mark criteria as completed
