# Issue #42 Comprehension - Concurrent Write Data Loss

## Problem Statement

### What the user is experiencing
Users are experiencing intermittent data loss when multiple API workers update the same user profile simultaneously. Specifically, when two concurrent requests attempt to update different fields of the same user record, sometimes only one of the updates persists.

### Expected behavior
When two API workers update different fields of the same user profile:
- Worker A sets `email=new@example.com`
- Worker B sets `phone=555-1234`
- Expected result: Both `email` and `phone` fields are updated in the final persisted state

### Actual behavior
Sometimes only one of the two updates is persisted. Either:
- Only the email update persists (phone unchanged), OR
- Only the phone update persists (email unchanged)

The lost update appears to happen silently with no error messages or exceptions logged.

## Key Details

### Environment
- **Deployment:** 4 API workers behind a load balancer
- **Storage:** Shared file-based storage system
- **Platform:** Python 3.11
- **Architecture:** Multiple processes accessing the same storage files

### Occurrence Pattern
- Happens approximately 2% of concurrent write attempts
- Only occurs when writes are truly concurrent (multiple workers writing simultaneously)
- Single-threaded tests pass fine (no data loss)
- Frequency increases under higher load

### What they've already tried
- Added retry logic to API workers - didn't help
- The problem isn't transient network issues or temporary failures
- The issue appears to be a fundamental concurrency problem

## Initial Hypotheses

### Most likely: Read-Modify-Write Race Condition
The file-based storage system likely uses a read-modify-write pattern:
1. Read the entire user record from disk
2. Modify the specific field in memory
3. Write the entire record back to disk

If two workers execute this sequence concurrently:
- Worker A reads record (email=old@example.com, phone=555-0000)
- Worker B reads record (email=old@example.com, phone=555-0000)
- Worker A modifies email, writes back (email=new@example.com, phone=555-0000)
- Worker B modifies phone, writes back (email=old@example.com, phone=555-1234)
- Result: Worker B's write overwrites Worker A's email change

### Other possibilities
- File locking not implemented or not working correctly
- Buffering issues causing writes to be reordered
- Cache coherency issues between workers
- Atomic write operations not being used

## Areas of Code to Investigate

1. **Storage Layer Implementation**
   - File: `src/storage/store.py` (expected location)
   - Look for: Read, modify, write operations
   - Check for: Locking mechanisms (or lack thereof)

2. **Concurrency Primitives**
   - Are file locks being used?
   - Are there any synchronization mechanisms?
   - How are concurrent writes handled?

3. **Write Operations**
   - How is the data serialized and written?
   - Are writes atomic?
   - Is there any transaction mechanism?

## Questions for Phase 2 (Reproduction)

### What would a reproduction test need to demonstrate?
1. **Concurrent execution**: Use threading or multiprocessing to simulate multiple workers
2. **Same key updates**: Multiple threads/processes updating the same user record
3. **Different field updates**: Each thread updates a different field
4. **Verification**: Check that ALL updates are present in the final state
5. **Reliability**: Test should fail consistently (not just 2% of the time)

### What concurrent scenarios should we test?
- Two concurrent writes to different fields of the same record
- Multiple (3+) concurrent writes
- Rapid succession writes (minimal delay)
- Writes with different payload sizes

### How can we make the test deterministic?
- Use synchronization primitives (barriers, events) to ensure truly concurrent execution
- Add artificial delays in the storage layer to widen the race window
- Run the test multiple times to ensure consistent failure
- Use a small dataset to increase collision probability

## Confidence Level

**High confidence** this is a classic read-modify-write race condition in the storage layer. The symptoms are textbook:
- Only happens under concurrent load
- Silent data loss (last write wins)
- Single-threaded tests work fine
- Shared file-based storage

Next step: Create a failing test that reproduces this reliably.
