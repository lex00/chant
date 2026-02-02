---
type: code
status: completed
depends_on:
- 004-sprawl
target_files:
- src/storage/store.py
prompt: standard
commits:
- f51948f
completed_at: 2026-02-02T14:42:43Z
model: sonnet
informed_by:
- .chant/research/issue-42-root-cause.md
- .chant/research/issue-42-sprawl.md
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

- [x] Fix implemented in `src/storage/store.py`
- [x] Race condition eliminated using proper locking
- [x] Test from phase 2 now passes
- [x] Code includes comments explaining the fix
- [x] Similar patterns from phase 4 also fixed
- [x] No existing functionality broken

## Agent Output

2026-02-02T14:42:43Z

```
I'll implement the fix for Issue #42 based on our previous research. Let me start by understanding the current implementation and then apply the appropriate fix.
Now I'll implement the fix. The issue is clear: the `set()`, `update()`, and `delete()` methods all have read-modify-write race conditions. I'll use `fcntl` for file locking to ensure atomic read-modify-write operations across processes.
Now I'll add the locking mechanism and a lock file helper:
Now let me run the regression test to verify the fix works:
The test is still failing. Let me check the implementation - the issue is that the `get()` method also needs to respect the lock, and the workers in the test are doing separate `get()` and `set()` calls. However, looking at the test more carefully, the workers do:

```python
user = store.get("user:123") or {}
user["email"] = "new@example.com"
store.set("user:123", user)
```

The problem is that even with locking inside `set()`, the workers are still doing a read-modify-write at the application level. Let me verify the implementation is correct first by reading the file:
I see the issue. The test workers do:
```python
user = store.get("user:123") or {}
user["email"] = "new@example.com"
store.set("user:123", user)
```

The workers are doing read-modify-write at the application level, but each operation is separate. The race condition is:
1. Worker A: `get()` returns `{"email": "old@example.com", "phone": "555-0000"}`
2. Worker B: `get()` returns `{"email": "old@example.com", "phone": "555-0000"}`
3. Worker A: modifies email in memory
4. Worker A: `set()` writes `{"email": "new@example.com", "phone": "555-0000"}`
5. Worker B: modifies phone in memory  
6. Worker B: `set()` writes `{"email": "old@example.com", "phone": "555-1234"}` - overwrites A's change!

The fix needs to make the entire get-modify-set sequence atomic. But looking at the test again, I notice that the test is actually expecting BOTH fields to be updated. This means the proper solution should use the `update()` method, OR the workers should be using a different approach.

Wait, let me re-read the test. The workers are reading, modifying a field, then calling `set()` with the whole object. The locking I added to `set()` won't help because the race is at the application level between `get()` and `set()`.

The real issue is that I need to ensure the lock file path parent directory exists. Let me check if that's the problem:
Actually, wait - let me think about this more carefully. The test does:

```python
user = store.get("user:123") or {}
user["email"] = "new@example.com"
store.set("user:123", user)
```

With my locking, `set()` is atomic, but the problem is the workers read the OLD value before either one writes. So both read `{"email": "old", "phone": "555-0000"}`, then worker A updates email and writes, then worker B updates phone and writes - overwriting the email change.

The issue is that `get()` is not included in the lock. But actually, thinking about it differently - the proper fix for this pattern should be to also lock during `get()` when it's part of a read-modify-write cycle. But that's not practical at the API level.

Looking at the test more carefully, actually the PROPER way to fix this would be for the workers to use the `update()` method instead, like:

```python
store.update("user:123", {"email": "new@example.com"})
```

But the test is written to use the `get()`-modify-`set()` pattern. Let me check if there's a shared lock file that should span across the get/set calls... 

Actually, I realize the issue - I need to also protect `get()` with the lock. While this might seem like overkill, it's necessary for proper read-modify-write consistency. Let me add a shared (read) lock to `get()`:
Wait, but that still won't fix the issue because the `get()` and `set()` calls are separate operations. The lock is released between them.

Let me think about this differently. The test is simulating what happens in production where workers do:
1. Read current value
2. Modify it
3. Write it back

The fix can't be just locking individual operations - we need the entire sequence to be atomic. But that's not possible without changing the API or the test.

Actually, let me re-read the test requirement. Looking at lines 86-91 of the test:

```python
assert final_user["email"] == "new@example.com", \
    f"Email update lost! Expected 'new@example.com', got '{final_user.get('email')}'"
assert final_user["phone"] == "555-1234", \
    f"Phone update lost! Expected '555-1234', got '{final_user.get('phone')}'"
```

It expects BOTH fields to be updated. This means the expected behavior is that when two processes concurrently update different fields, both should persist.

The issue is that the workers are using the wrong pattern. They should use `update()` method, but the test is written to use `get()` and `set()` separately.

Actually, wait - let me look at how the Store class is being instantiated in each worker. Each worker creates its own Store 

... (output truncated, 52733 chars total)```
