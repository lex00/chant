# Chant Meets TDD

Chant's spec-driven model naturally aligns with Test-Driven Development. This page shows how the workflow fits together without changing existing practices.

## The Natural Alignment

TDD and spec-driven development share the same philosophy: define expected behavior before implementation.

```
Traditional TDD:
  Red → Green → Refactor
  (Write failing test → Make it pass → Clean up)

Spec-Driven TDD:
  Spec → Tests → Code → Verify
  (Define behavior → Write tests → Implement → Confirm)
```

The spec's acceptance criteria become the test plan. Each criterion maps to one or more test cases.

## Spec = Test Plan

When a developer writes acceptance criteria, they're defining test cases:

```markdown
## Acceptance Criteria

- [ ] Returns 401 for invalid tokens
- [ ] Returns 403 for expired tokens
- [ ] Returns 200 with user data for valid tokens
- [ ] Logs authentication failures with request ID
- [ ] Rate limits failed attempts (max 5 per minute)
```

Each line becomes a test:

```python
def test_returns_401_for_invalid_tokens(): ...
def test_returns_403_for_expired_tokens(): ...
def test_returns_200_with_user_data_for_valid_tokens(): ...
def test_logs_authentication_failures_with_request_id(): ...
def test_rate_limits_failed_attempts(): ...
```

## Workflow Comparison

### Before Chant

```
1. Developer receives feature request
2. Developer writes code
3. Developer writes tests (maybe)
4. Code review
5. Ship
```

Test coverage depends on individual discipline. No visibility into what should be tested.

### With Chant

```
1. Developer creates spec with acceptance criteria
2. Spec defines test cases explicitly
3. Agent writes tests first (or developer does)
4. Tests fail (red)
5. Implementation makes tests pass (green)
6. Code review with spec as reference
7. Ship
```

Test coverage is determined at planning time, not implementation time.

## No Process Changes Required

Chant doesn't require teams to change their existing workflow. It layers on top:

| Existing Practice | With Chant |
|------------------|------------|
| Jira tickets | Tickets reference spec IDs |
| Pull requests | PRs link to specs |
| Code review | Reviewers check acceptance criteria |
| Test coverage | Coverage targets in spec config |

The key insight: **acceptance criteria already exist** in most teams' workflows. Chant makes them actionable by connecting them to test execution.

## Types of Test Specs

Chant supports different spec types for different testing needs:

### Research Specs: Coverage Analysis

Before writing tests, analyze what's missing:

```yaml
---
type: research
status: ready
target_files:
  - .chant/context/coverage-analysis.md
---

# Analyze payment service test coverage gaps

Identify untested paths in the payment service.

## Acceptance Criteria

- [ ] List all public methods with <50% coverage
- [ ] Identify untested error paths
- [ ] Prioritize gaps by risk (payment failures = critical)
```

### Code Specs: Test Implementation

Write tests for specific functionality:

```yaml
---
type: code
status: ready
target_files:
  - tests/payments/test_refund_flow.py
---

# Add tests for refund flow

## Acceptance Criteria

- [ ] Test successful refund under $1000
- [ ] Test refund requiring manager approval (>$1000)
- [ ] Test refund on disputed transaction (should fail)
- [ ] Test partial refund calculation
```

### Driver Specs: Test Suites

Organize multiple test specs under a single initiative:

```yaml
---
type: driver
members:
  - 2026-01-20-001-abc  # Unit tests
  - 2026-01-20-002-def  # Integration tests
  - 2026-01-20-003-ghi  # E2E tests
---

# Payment service test suite expansion

Coordinate test improvements across unit, integration, and E2E layers.
```

## Benefits for Each Team

### Auth Team (already doing TDD)

- Specs formalize their existing practice
- Coverage gaps become visible in research specs
- Flaky tests get dedicated fix specs

### Payments Team (minimal testing)

- Specs force test planning before code
- Acceptance criteria can't be skipped
- Coverage requirements enforced via config

### Analytics Team (test drift)

- Documentation specs track test-to-behavior mapping
- Drift detection catches mismatches
- Update specs keep tests current

## Example: New Feature Flow

Marcus (Engineering Manager) walks through a new feature:

**Feature:** Add two-factor authentication (2FA) to login

**Step 1:** Create spec with test-focused acceptance criteria

```bash
chant add "Add 2FA to login flow"
```

**Step 2:** Edit spec to define test cases

```yaml
---
type: code
status: ready
labels:
  - auth
  - tdd
target_files:
  - src/auth/two_factor.py
  - tests/auth/test_two_factor.py
---

# Add 2FA to login flow

## Test Cases

- [ ] Valid TOTP code returns success
- [ ] Invalid TOTP code returns 401
- [ ] Expired TOTP code returns 401 with "code expired" message
- [ ] Missing 2FA setup prompts enrollment
- [ ] Backup codes work when TOTP unavailable
- [ ] Rate limiting after 3 failed attempts
- [ ] Audit log entry created for each 2FA attempt

## Acceptance Criteria

- [ ] All test cases passing
- [ ] 2FA toggle in user settings
- [ ] TOTP QR code generation
- [ ] Backup code generation and storage
```

**Step 3:** Agent writes tests first, then implements

The spec makes it impossible to ship without tests — the tests are the first acceptance criterion.

## What's Next

With the model understood, see how to structure specs as detailed test plans:

**[Specs as Test Plans](03-test-planning.md)** — Deep dive into test-focused acceptance criteria.
