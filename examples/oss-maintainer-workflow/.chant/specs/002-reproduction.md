---
type: code
status: pending
prompt: standard
depends_on:
  - 001-comprehension
informed_by:
  - .chant/research/issue-42-comprehension.md
target_files:
  - tests/regression/test_issue_42.py
---
# Phase 2: Reproduction - Create failing test for Issue #42

## Context

Based on phase 1 comprehension, we understand that:
- Two concurrent writes to the same key cause data loss
- No errors are logged
- Happens ~2% of the time under concurrent load
- Single-threaded tests work fine

## Task

Create a failing test at `tests/regression/test_issue_42.py` that demonstrates the concurrent write data loss bug.

The test should:
1. Simulate two concurrent writes to the same storage key
2. Verify that both writes are persisted (this will fail until the bug is fixed)
3. Use threading or multiprocessing to create realistic concurrency
4. Be deterministic enough to fail reliably (not just 2% of the time)

## Test Requirements

```python
# Should test concurrent updates to the same key
# Should verify both updates are present in final state
# Should fail with current implementation
# Should pass once the bug is fixed
```

## Acceptance Criteria

- [ ] Test file created at `tests/regression/test_issue_42.py`
- [ ] Test uses concurrent execution (threading/multiprocessing)
- [ ] Test writes to the same key from multiple threads/processes
- [ ] Test verifies both writes are persisted
- [ ] Test currently fails (demonstrating the bug)
- [ ] Test includes clear comments explaining what it's checking
