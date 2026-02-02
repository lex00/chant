---
type: code
status: completed
labels:
  - payments
  - tdd
  - unit
parent: 002-test-suite-driver
target_files:
  - tests/payments/test_currency_conversion.py
completed_at: 2026-01-20T11:05:00Z
model: claude-sonnet-4-5-20250929
---

# Add tests for currency conversion in refunds

## Problem

International refunds (18% of total volume) have only 28% test coverage.
Production incident in December: incorrect exchange rate caused $3,200 discrepancy.

## Test Categories

### Currency Conversion

- [x] Refund in original currency (EUR → EUR)
- [x] Refund with USD conversion applies current rate
- [x] Exchange rate recorded in refund record
- [x] Multi-currency partial refunds maintain correct balances

## Acceptance Criteria

- [x] All 4 test cases implemented and passing
- [x] Currency conversion logic covered
- [x] Exchange rate mocking properly isolated
- [x] No flaky tests (verified with 10 consecutive runs)
