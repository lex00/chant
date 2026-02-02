---
type: code
status: completed
prompt: standard
depends_on:
  - 002-test-suite-driver
target_files:
  - tests/payments/test_refund_flow.py
---

# Add comprehensive tests for payment refund flow

## Problem

The refund flow has 38% test coverage. Production incidents in Q4 revealed
untested edge cases: partial refunds, currency conversion, and fraud holds.

## Test Categories

### Happy Path Tests

- [x] Full refund on completed transaction
- [x] Partial refund with correct remaining balance
- [x] Refund to original payment method
- [x] Refund confirmation email triggered

### Authorization Tests

- [x] Refund under $100: auto-approved
- [x] Refund $100-$1000: requires team lead approval
- [x] Refund over $1000: requires manager approval
- [x] Refund on flagged account: requires fraud review

### Edge Cases

- [x] Refund on transaction older than 90 days (policy limit)
- [x] Refund exceeds original transaction amount (reject)
- [x] Multiple partial refunds totaling more than original (reject)
- [x] Refund on disputed transaction (blocked)

### Error Handling

- [x] Invalid transaction ID returns 404
- [x] Insufficient balance returns 400 with clear message
- [x] Payment processor timeout triggers retry with backoff
- [x] Database error triggers rollback, no partial state

## Acceptance Criteria

- [x] All 16 test cases implemented and passing
- [x] Test fixtures created in conftest.py
- [x] Coverage of refund module reaches 85%+
- [x] No flaky tests (verified with 10 consecutive runs)
