---
type: code
status: completed
prompt: standard
depends_on:
  - 002-test-suite-driver
target_files:
  - tests/payments/test_refund_retry.py
---

# Add tests for refund retry logic

## Problem

Payment processor outages caused 43 failed refunds in December. Retry logic
exists but has 0% test coverage.

## Test Categories

### Retry Logic

- [x] Processor timeout queues refund for retry with 30s backoff
- [x] Second retry uses 60s backoff (exponential)
- [x] After 5 retries, refund marked as failed
- [x] Successful retry on second attempt clears retry count

## Acceptance Criteria

- [x] All 4 test cases implemented and passing
- [x] Exponential backoff logic verified
- [x] Max retry limit enforced
- [x] No flaky tests (verified with 10 consecutive runs)
