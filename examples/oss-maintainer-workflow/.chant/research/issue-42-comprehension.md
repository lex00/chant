# Investigation: Concurrent Write Data Loss in Storage Layer (Issue #42)

## Problem Statement

**Observed behavior**: When two API workers update different fields of the same user profile simultaneously, sometimes only one update persists. The other update is lost without any error being logged.

**Expected behavior**: Both updates should persist. If Worker A updates the email field and Worker B updates the phone field for the same user simultaneously, both fields should reflect the new values.

**Context**:
- Occurs in production with 4 API workers behind a load balancer
- Happens approximately 2% of the time during concurrent write attempts
- No errors are logged when data loss occurs
- Single-threaded tests pass fine

## Evidence Collected

### Code Analysis

Key files examined:

**`src/storage/store.py:31-42`** — The `set()` method demonstrates a classic read-modify-write race condition:
```python
def set(self, key, value):
    data = self._read_all()      # Read entire file
    data[key] = value             # Modify in memory
    self._write_all(data)         # Write entire file
    # If another process writes between read and write, their changes are lost!
```

**`src/storage/store.py:44-58`** — The `update()` method has the same race condition pattern

**`src/storage/store.py:70-75`** — The `_read_all()` method reads the entire JSON file

**`src/storage/store.py:77-83`** — The `_write_all()` method uses atomic file replacement (write to temp, then rename), which prevents file corruption but does NOT prevent race conditions

**`tests/regression/test_issue_42.py:20-43`** — Two worker processes demonstrate the concurrent update scenario using a multiprocessing barrier to synchronize timing

### Architecture

The storage system is a file-based key-value store:
- All data stored in a single JSON file
- Each write operation: reads entire file → modifies in memory → writes entire file
- File writes are atomic (temp file + rename) preventing corruption
- No locking mechanism between read and write operations
- Multiple processes (API workers) share access to the same storage file

### Reproduction Case

The test case `test_concurrent_writes_both_persist()` in `tests/regression/test_issue_42.py` reproduces the issue:

1. Initialize user:123 with `{"email": "old@example.com", "phone": "555-0000"}`
2. Worker A and Worker B synchronize using a barrier
3. Both workers simultaneously:
   - Read the current user data
   - Worker A modifies email to "new@example.com"
   - Worker B modifies phone to "555-1234"
   - Both write their modified versions
4. Result: Whichever worker writes last overwrites the other's changes

## Hypotheses Considered

### Hypothesis 1: File Corruption from Non-Atomic Writes

**Supporting evidence**: Multiple processes writing to the same file could cause corruption

**Testing**: Examined `_write_all()` implementation at `src/storage/store.py:77-83`

**Result**: ELIMINATED - The code uses atomic file replacement (write to temp file, then rename), which prevents file corruption. The user also reported no file corruption, only data loss.

### Hypothesis 2: Race Condition in Read-Modify-Write Pattern

**Supporting evidence**:
- The `set()` and `update()` methods follow a read-modify-write pattern with no locking
- The issue only occurs under concurrent load (single-threaded tests pass)
- The timing window between read and write allows interleaving

**Testing**: Traced execution flow for concurrent scenario:
1. Worker A reads file (gets original data)
2. Worker B reads file (gets original data)
3. Worker A modifies data in memory
4. Worker B modifies data in memory
5. Worker A writes modified data to file
6. Worker B writes modified data to file (overwrites A's changes)

**Result**: CONFIRMED - This is the root cause

### Hypothesis 3: Retry Logic Could Prevent Data Loss

**Supporting evidence**: User mentioned trying retry logic

**Testing**: Analyzed the nature of the bug - retrying after a failed write wouldn't help because:
- No errors are generated (the write succeeds)
- The data loss is silent - the last writer wins
- Retrying would just repeat the same race condition

**Result**: CONFIRMED - Retry logic cannot fix this issue, which aligns with the user's experience

## Root Cause

**Location**: `src/storage/store.py:31-42` (and similarly in `update()` at lines 44-58 and `delete()` at lines 60-68)

**Explanation**: The storage layer implements a read-modify-write pattern without any synchronization mechanism to prevent concurrent access. This creates a race condition where multiple processes can read the same data, make independent modifications, and write back their changes - causing the last writer to overwrite all previous writes.

**Mechanism**: Step-by-step explanation of how data loss occurs:

1. **T0**: Worker A receives request to update `user:123.email` to "new@example.com"
2. **T1**: Worker B receives request to update `user:123.phone` to "555-1234"
3. **T2**: Worker A calls `set()` and executes `_read_all()`, loading `{"email": "old@example.com", "phone": "555-0000"}`
4. **T3**: Worker B calls `set()` and executes `_read_all()`, loading the same `{"email": "old@example.com", "phone": "555-0000"}`
5. **T4**: Worker A modifies its in-memory copy to `{"email": "new@example.com", "phone": "555-0000"}`
6. **T5**: Worker B modifies its in-memory copy to `{"email": "old@example.com", "phone": "555-1234"}`
7. **T6**: Worker A calls `_write_all()`, writing `{"email": "new@example.com", "phone": "555-0000"}` to disk
8. **T7**: Worker B calls `_write_all()`, writing `{"email": "old@example.com", "phone": "555-1234"}` to disk
9. **Result**: Worker A's email update is lost because Worker B overwrote it with the old email value

**Conditions**: The issue occurs when:
- Two or more processes attempt to write to the same key concurrently
- The timing is such that both processes read before either writes (window between T2-T6)
- Under 4 API workers with load balancing, the 2% occurrence rate suggests the timing window is relatively small but happens frequently enough to be problematic

## Impact

**Who is affected**: Any production system using this storage layer with multiple API workers

**Conditions**:
- Multiple concurrent requests updating the same resource
- Higher frequency with more workers and higher load
- Silent failures (no errors logged), making it difficult to detect and debug

**Potential consequences**:
- Data integrity violations
- Inconsistent state in production
- User-visible bugs (missing profile updates, lost settings, etc.)
- Difficult to reproduce and debug due to timing-dependent nature
- Customer trust issues due to "lost" updates

## Related Issues

**Similar patterns in the codebase**:
- `update()` method at `src/storage/store.py:44-58` has identical race condition
- `delete()` method at `src/storage/store.py:60-68` has identical race condition
- Any method following the read-modify-write pattern is vulnerable

**Potential related issues**:
- If other parts of the codebase use this Store class, they all inherit this vulnerability
- The 2% failure rate suggests there may be more frequent but unnoticed data loss in lower-traffic scenarios

## Recommendations

### Primary Fix: Add File Locking

Implement file locking to prevent concurrent read-modify-write operations:

**Approach**: Use `fcntl.flock()` (Unix) or equivalent to lock the storage file during read-modify-write operations

**Why this approach**:
- Directly addresses the root cause by preventing concurrent access
- Minimal code changes required
- Works across multiple processes (which is the current production scenario)
- Industry-standard solution for file-based coordination

**What it would change**:
- Wrap read-modify-write sequences in lock acquisition/release
- Add lock context manager for clean error handling
- All write operations (`set()`, `update()`, `delete()`) would acquire exclusive lock
- Read operations could use shared locks if read consistency is needed

**Potential risks**:
- Performance impact: serializes all write operations to the same storage file
- Deadlock potential if not implemented carefully (mitigated by using context managers and timeouts)
- Platform-specific considerations (Windows uses different locking mechanism)

### Alternative Fix: Move to Database with Transactions

Replace file-based storage with a proper database (SQLite, PostgreSQL, etc.)

**Why this approach**:
- Databases have built-in transaction support and concurrency control
- Better performance characteristics at scale
- More robust error handling
- Industry standard for multi-process data access

**Trade-offs vs primary**:
- Much larger change (requires migration, schema design, dependency addition)
- May be overengineering if the system requirements are simple
- Breaking change for the API
- Better long-term solution but higher implementation cost

### Alternative Fix: Optimistic Locking with Version Numbers

Add version numbers to detect conflicting updates:

**Why this approach**:
- Allows detection of conflicts without blocking
- Can return errors to callers who can retry with fresh data
- More scalable than pessimistic locking

**Trade-offs vs primary**:
- Requires changing data model (add version field)
- Requires callers to handle conflict errors
- Doesn't prevent the conflict, just detects it
- More complex implementation

## Next Steps

What should happen after this investigation:

- [x] Document findings in this research file
- [ ] Create spec for implementing the fix (likely file locking solution)
- [ ] Create spec for validating fix passes regression tests
- [ ] Consider creating spec for stress testing under higher concurrency
- [ ] Consider creating spec for adding logging/monitoring to detect future data loss

## Questions for Phase 2 (Reproduction)

Since we already have a reproduction test (`tests/regression/test_issue_42.py`), Phase 2 would focus on:

1. **Validation**: Can we confirm the current test reliably reproduces the bug?
2. **Edge cases**: What other concurrent scenarios should we test?
   - Concurrent deletes?
   - Mix of reads and writes?
   - Three or more concurrent writers?
3. **Fix verification**: How do we ensure the fix works under stress?
   - The stress test at `test_concurrent_writes_stress()` provides this
4. **Performance**: What is the performance impact of the locking solution?
   - Should we benchmark locked vs unlocked implementation?
