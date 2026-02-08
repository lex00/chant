# Ensuring Quality Across Teams

With three teams writing tests differently, Marcus needs to enforce consistent standards. Chant's configuration and validation features help maintain quality without micromanagement.

## The Consistency Problem

Before standardization:

| Team | Test Naming | Fixture Pattern | Assertions/Test | Coverage Threshold |
|------|------------|-----------------|-----------------|-------------------|
| Auth | `test_*` | Factories | 3-5 | 80% |
| Payments | `it_should_*` | Raw data | 1-2 | None |
| Analytics | `test_*` | Fixtures | 2-4 | 90% |

Different patterns make tests harder to review, maintain, and debug.

## Config-Based Standards

Marcus creates a shared configuration that enforces test requirements:

**File: `.chant/config.md`**

```markdown
## spec_defaults

### code
- target_files: required
- test_coverage_target: 80

### required_labels
- One of: auth, payments, analytics (team identifier)
- One of: unit, integration, e2e (test level)

## validation

### coverage
- minimum: 80
- warn_below: 85
- modules:
  - path: "payments/*"
    minimum: 85
  - path: "auth/*"
    minimum: 80

### test_quality
- min_assertions_per_test: 2
- max_test_file_length: 500
- require_docstrings: true
```

## Required Fields

Spec validation catches incomplete test specs before execution:

```bash
chant lint
```

```
Linting 12 specs...

WARN  2026-01-21-001-xyz: Missing required label (team identifier)
ERROR 2026-01-21-002-abc: target_files is required for code specs
ERROR 2026-01-21-003-def: Test coverage target not specified

2 errors, 1 warning
```

Specs with errors can't be executed:

```bash
chant work 2026-01-21-002-abc
```

```
Error: Spec 2026-01-21-002-abc failed validation
  - target_files is required for code specs

Fix validation errors before executing.
```

## Test Spec Template

Marcus creates a template that pre-fills required fields:

**File: `.chant/templates/test-spec.md`**

```markdown
---
type: code
status: pending
labels:
  - {{team}}
  - {{test_level}}
target_files:
  - tests/{{module}}/test_{{feature}}.py
---

# Add tests for {{feature}}

## Problem

Brief description of what needs testing and why.

## Test Categories

### Happy Path
- [ ] Test case 1
- [ ] Test case 2

### Edge Cases
- [ ] Edge case 1
- [ ] Edge case 2

### Error Handling
- [ ] Error case 1
- [ ] Error case 2

## Acceptance Criteria

- [ ] All test cases passing
- [ ] Coverage target met (80%+)
- [ ] No flaky tests (verified with 10 runs)
- [ ] Tests follow naming convention
```

Creating a test spec with the template:

```bash
chant add --template test-spec \
  --var team=payments \
  --var test_level=unit \
  --var module=refund \
  --var feature=partial_refund \
  "Add unit tests for partial refund"
```

## Context Files for Standards

Store team-wide test patterns in context files:

**File: `.chant/context/tdd-standards/naming-conventions.md`**

```markdown
# Test Naming Conventions

## Test Functions

Pattern: `test_<action>_<condition>_<expected_result>`

Good:
- `test_refund_exceeds_original_returns_400`
- `test_login_expired_token_returns_401`
- `test_subscription_canceled_stops_billing`

Bad:
- `test_refund` (too vague)
- `test_it_works` (meaningless)
- `testRefundFlow` (wrong style)

## Test Classes

Pattern: `Test<Component><Category>`

Examples:
- `TestRefundHappyPath`
- `TestRefundEdgeCases`
- `TestRefundAuthorization`
```

**File: `.chant/context/tdd-standards/fixture-patterns.md`**

```markdown
# Fixture Patterns

## Use Factories Over Raw Data

Good:
```python
@pytest.fixture
def completed_transaction():
    return TransactionFactory(status="completed", amount=Decimal("100.00"))
```

Bad:
```python
@pytest.fixture
def completed_transaction():
    return {"id": 1, "status": "completed", "amount": 100}
```

## Mock External Services at Client Boundary

Good:
```python
@patch("payments.stripe_client.StripeClient.charge")
def test_payment_processed(mock_charge):
    mock_charge.return_value = ChargeResult(success=True)
```

Bad:
```python
@patch("stripe.Charge.create")  # Too deep
def test_payment_processed(mock_create):
```
```

## Automated Quality Checks

Marcus sets up a CI job that runs quality checks:

**File: `.github/workflows/test-quality.yml`**

```yaml
name: Test Quality

on: [push, pull_request]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Lint specs
        run: chant lint --strict

      - name: Check test coverage thresholds
        run: |
          pytest --cov=payments --cov=auth --cov=analytics \
            --cov-fail-under=80

      - name: Check for flaky tests
        run: pytest --count=5 -x

      - name: Validate test patterns
        run: |
          # Check naming conventions
          grep -r "def test_" tests/ | \
            grep -v "def test_[a-z_]*_[a-z_]*" && \
            echo "ERROR: Tests not following naming convention" && exit 1 || true
```

## Team-Specific Overrides

While enforcing minimums, teams can exceed them:

**File: `.chant/context/tdd-standards/team-overrides.md`**

```markdown
# Team-Specific Standards

## Auth Team

Higher coverage required due to security sensitivity:
- Minimum coverage: 85% (vs 80% baseline)
- All auth flows require integration tests
- Session handling requires E2E tests

## Payments Team

Critical path coverage:
- Transaction processing: 90% minimum
- Refund flow: 85% minimum
- Error handling: explicit tests for all error codes

## Analytics Team

Performance-sensitive tests:
- Query tests must assert execution time
- Dashboard tests must verify caching
```

## Validation Report

Regular validation reports track compliance:

```bash
# Note: Use activity and export commands for reporting
chant activity --since 30d
chant export --format json
```

```
Test Quality Report — Last 30 Days

Team Coverage Compliance:
  Auth:      87% (target: 85%) ✓
  Payments:  86% (target: 85%) ✓
  Analytics: 91% (target: 80%) ✓

Flaky Test Rate:
  Auth:      3% (target: <5%) ✓
  Payments:  4% (target: <5%) ✓
  Analytics: 2% (target: <5%) ✓

Test Specs Created: 23
Test Specs Completed: 21
Tests Added: 156

Standards Compliance:
  Naming convention: 98% (3 violations)
  Min assertions: 100%
  Docstrings: 94% (9 missing)
```

## Before/After: Team Consistency

After implementing standards:

| Metric | Before | After |
|--------|--------|-------|
| Consistent naming | 65% | 98% |
| Factory usage | 40% | 95% |
| Docstring coverage | 30% | 94% |
| Meeting coverage target | 1/3 teams | 3/3 teams |

## What's Next

With standards enforced, see how to keep tests current as code evolves:

**[Keeping Tests Current](06-drift-detection.md)** — Detecting and fixing test drift.
