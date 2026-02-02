---
type: code
status: pending
depends_on:
- 004-sprawl
informed_by:
- .chant/research/issue-42-root-cause.md
- .chant/research/issue-42-sprawl.md
target_files:
- src/storage/store.py
---
# Phase 5: Fork Fix - Implement fix for Issue #42

## Context

We have completed our research:
- Phase 3 identified the root cause: read-modify-write race condition
- Phase 4 assessed the scope: found 2 similar patterns in the codebase

Now we implement the fix in our fork before creating an upstream PR.

## Task

Fix the concurrent write data loss bug in `src/storage/store.py` based on the root cause analysis.

Implementation should:
1. Use file locking or atomic operations to prevent race conditions
2. Fix the primary location identified in phase 3
3. Fix any similar patterns identified in phase 4
4. Make the test from phase 2 pass
5. Preserve backward compatibility if possible

## Fix Requirements

- Use appropriate locking mechanism (fcntl, threading.Lock, etc.)
- Ensure both concurrent writes are persisted
- Don't break existing single-threaded behavior
- Add comments explaining the locking strategy
- Consider performance impact

## Testing

After implementing the fix:
1. Run `tests/regression/test_issue_42.py` - should now PASS
2. Verify no existing tests broke
3. Test under high concurrency to verify fix holds

## Acceptance Criteria

- [ ] Fix implemented in `src/storage/store.py`
- [ ] Race condition eliminated using proper locking
- [ ] Test from phase 2 now passes
- [ ] Code includes comments explaining the fix
- [ ] Similar patterns from phase 4 also fixed
- [ ] No existing functionality broken
