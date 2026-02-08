# Specs as Test Plans

Specs become test plans when acceptance criteria are written as testable assertions. This page shows how to structure specs for effective TDD.

## The Test Planning Spec

The Payments team needs to improve coverage for their refund processing flow. Marcus creates a comprehensive test planning spec.

### Creating the Spec

```bash
chant add "Add comprehensive tests for payment refund flow"
```

### Complete Test Planning Spec

**File: `.chant/specs/2026-01-20-001-rfn.md`**

```yaml
---
type: code
status: ready
labels:
  - payments
  - tdd
  - coverage
target_files:
  - tests/payments/test_refund_flow.py
  - tests/payments/test_refund_edge_cases.py
  - tests/payments/conftest.py
---

# Add comprehensive tests for payment refund flow

## Problem

The refund flow has 38% test coverage. Production incidents in Q4 revealed
untested edge cases: partial refunds, currency conversion, and fraud holds.

## Test Categories

### Happy Path Tests

- [ ] Full refund on completed transaction
- [ ] Partial refund with correct remaining balance
- [ ] Refund to original payment method
- [ ] Refund confirmation email triggered

### Authorization Tests

- [ ] Refund under $100: auto-approved
- [ ] Refund $100-$1000: requires team lead approval
- [ ] Refund over $1000: requires manager approval
- [ ] Refund on flagged account: requires fraud review

### Edge Cases

- [ ] Refund on transaction older than 90 days (policy limit)
- [ ] Refund exceeds original transaction amount (reject)
- [ ] Multiple partial refunds totaling more than original (reject)
- [ ] Refund during payment processor outage (queue for retry)
- [ ] Currency conversion on international refund
- [ ] Refund on disputed transaction (blocked)

### Error Handling

- [ ] Invalid transaction ID returns 404
- [ ] Insufficient balance returns 400 with clear message
- [ ] Payment processor timeout triggers retry with backoff
- [ ] Database error triggers rollback, no partial state

## Acceptance Criteria

- [ ] All 16 test cases implemented and passing
- [ ] Test fixtures created in conftest.py
- [ ] Coverage of refund module reaches 85%+
- [ ] No flaky tests (verified with 10 consecutive runs)
```

This single spec defines 16 test cases organized by category. Each checkbox becomes a test function.

## Research Specs for Coverage Analysis

Before writing tests, understand what's missing. Research specs analyze existing coverage.

### Coverage Analysis Spec

```bash
chant add "Analyze payment service test coverage gaps" --type research
```

**File: `.chant/specs/2026-01-18-001-cov.md`**

```yaml
---
type: research
status: ready
labels:
  - payments
  - tdd
informed_by:
  - .chant/context/tdd-standards/coverage-requirements.md
target_files:
  - .chant/context/payments-coverage-analysis.md
---

# Analyze payment service test coverage gaps

## Research Questions

- [ ] Which modules have <50% coverage?
- [ ] Which error paths are untested?
- [ ] Which edge cases appear in production logs but lack tests?
- [ ] What's the coverage by risk level (critical/high/medium)?

## Methodology

1. Parse coverage report for payments module
2. Cross-reference with production error logs (last 30 days)
3. Identify untested paths that caused incidents
4. Prioritize by business impact

## Acceptance Criteria

- [ ] Coverage gaps documented with line-level detail
- [ ] Gaps prioritized by risk (payment failures = P0)
- [ ] Recommended test additions with estimated effort
- [ ] Findings in .chant/context/payments-coverage-analysis.md
```

### Research Output

After execution, the agent produces a coverage analysis:

**File: `.chant/context/payments-coverage-analysis.md`**

```markdown
# Payment Service Coverage Analysis

Spec: 2026-01-18-001-cov
Completed: 2026-01-19

## Executive Summary

Payment service has 45% overall coverage. Critical paths (transaction
processing) have 72% coverage, but error handling paths have only 18%.

## Coverage by Module

| Module | Coverage | Risk Level | Gap Priority |
|--------|----------|------------|--------------|
| refund.py | 38% | Critical | P0 |
| transaction.py | 72% | Critical | P1 |
| subscription.py | 51% | High | P1 |
| reporting.py | 89% | Low | P3 |

## Critical Gaps (P0)

### refund.py (38% → target 85%)

| Line Range | Description | Production Incidents |
|------------|-------------|---------------------|
| 45-67 | Partial refund calculation | 3 in Dec |
| 102-118 | Currency conversion | 1 in Dec |
| 145-160 | Fraud hold handling | 2 in Nov |
| 189-205 | Retry on processor failure | 4 in Dec |

### Recommended Test Additions

1. **Partial refund flow** (8 tests, ~2 hours)
2. **Currency conversion** (4 tests, ~1 hour)
3. **Fraud hold scenarios** (3 tests, ~1 hour)
4. **Retry logic** (4 tests, ~1.5 hours)

Total: 19 new tests, estimated 5.5 hours
```

## Driver Specs for Test Suites

Large testing initiatives use driver specs to organize multiple test specs.

### Test Suite Driver

```yaml
---
type: driver
status: ready
labels:
  - payments
  - tdd
  - q1-coverage
members:
  - 2026-01-20-001-rfn  # Refund flow tests
  - 2026-01-20-002-cur  # Currency conversion tests
  - 2026-01-20-003-frd  # Fraud handling tests
  - 2026-01-20-004-rty  # Retry logic tests
---

# Payment service test coverage expansion

Based on coverage analysis in 2026-01-18-001-cov.

## Goal

Increase payment service coverage from 45% to 85%.

## Member Specs

| Spec | Focus | Tests | Status |
|------|-------|-------|--------|
| 001-rfn | Refund flow | 16 | Ready |
| 002-cur | Currency | 4 | Ready |
| 003-frd | Fraud | 3 | Ready |
| 004-rty | Retry | 4 | Ready |

## Acceptance Criteria

- [ ] All member specs completed
- [ ] Combined coverage reaches 85%
- [ ] No new flaky tests introduced
```

## Writing Effective Test Criteria

### Good: Specific and Testable

```markdown
- [ ] Refund exceeds original amount returns 400 with error code REFUND_EXCEEDS_ORIGINAL
- [ ] Rate limiting triggers after 3 failed attempts within 60 seconds
- [ ] Audit log contains user_id, action, timestamp, and result for each refund
```

### Bad: Vague or Untestable

```markdown
- [ ] Refund flow works correctly
- [ ] Error handling is robust
- [ ] Performance is acceptable
```

### Guidelines

1. **Include expected values** — "returns 400" not "returns error"
2. **Specify boundaries** — "after 3 attempts" not "rate limits requests"
3. **Name error codes** — "REFUND_EXCEEDS_ORIGINAL" not "appropriate error"
4. **Define data requirements** — "contains user_id, action, timestamp"

## Context Files for Test Standards

Store team test standards in context files:

**File: `.chant/context/tdd-standards/test-patterns.md`**

```markdown
# Acme Test Patterns

## Naming Convention

Tests follow: `test_<action>_<condition>_<expected_result>`

Examples:
- `test_refund_exceeds_original_returns_400`
- `test_login_invalid_password_increments_failure_count`
- `test_subscription_expired_blocks_api_access`

## Required Assertions

Every test must assert:
1. Return value or side effect
2. State change (if applicable)
3. Audit/logging (for security-sensitive operations)

## Fixture Requirements

- Use factories, not raw fixtures
- Mock external services at the client boundary
- Clean up database state after each test
```

Agents reference these standards when writing tests.

## What's Next

With test plans defined, see how agents execute them:

**[Writing Tests with Chant](04-execution.md)** — Agent-driven test implementation.
