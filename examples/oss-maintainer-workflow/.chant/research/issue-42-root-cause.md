# Issue #42 Root Cause Analysis

## The Buggy Code

**File:** `src/storage/store.py`
**Function:** `Store.set(key, value)`
**Lines:** 23-28

```python
def set(self, key, value):
    """Set a value in the store."""
    data = self._read_all()      # Line 24: Read entire file
    data[key] = value             # Line 25: Modify in memory
    self._write_all(data)         # Line 26: Write entire file
```

### The Problematic Pattern

This is a classic **read-modify-write race condition** without any locking:

1. `_read_all()` reads the entire storage file into memory
2. The in-memory dictionary is modified
3. `_write_all()` writes the entire dictionary back to disk

**No synchronization exists between these three steps.**

## The Race Condition

### Timing Diagram

```
Time    Worker A                          Worker B
----    --------                          --------
T0      set("email", "new@example.com")
T1      data_a = _read_all()
        # data_a = {email: "old", phone: "555-0000"}
T2                                        set("phone", "555-1234")
T3                                        data_b = _read_all()
                                          # data_b = {email: "old", phone: "555-0000"}
T4      data_a["email"] = "new@example.com"
        # data_a = {email: "new", phone: "555-0000"}
T5                                        data_b["phone"] = "555-1234"
                                          # data_b = {email: "old", phone: "555-1234"}
T6      _write_all(data_a)
        # File now: {email: "new", phone: "555-0000"}
T7                                        _write_all(data_b)
                                          # File now: {email: "old", phone: "555-1234"}
T8      # Worker B's write clobbered Worker A's email update!
```

### Why Data Loss Occurs

1. Both workers read the file before either has written
2. Each worker modifies their in-memory copy independently
3. The second write completely overwrites the first
4. Changes from the first write are lost (last write wins)

### Why Single-Threaded Tests Work

In single-threaded execution:
- Only one operation at a time
- read → modify → write completes before next operation starts
- No interleaving, no race condition

### Why It's Intermittent (~2% of writes)

The race condition only occurs when operations overlap:
- Read operations must overlap (both read before either writes)
- If operations are sufficiently separated in time, no collision
- 2% suggests operations are usually staggered enough to avoid the race

## Additional Vulnerable Locations

Scanning `src/storage/store.py` reveals the same pattern in:

**Function:** `Store.update(key, updates)` (lines 35-39)
```python
def update(self, key, updates):
    """Update multiple fields of a value."""
    data = self._read_all()           # Race condition!
    if key in data:
        data[key].update(updates)
    self._write_all(data)
```

**Function:** `Store.delete(key)` (lines 46-49)
```python
def delete(self, key):
    """Delete a key from the store."""
    data = self._read_all()           # Race condition!
    data.pop(key, None)
    self._write_all(data)
```

All three methods are vulnerable to the same race condition.

## The Fix Approach

### Option 1: File-Based Locking (Recommended)

Use `fcntl` (on Unix) or `msvcrt` (on Windows) to acquire an exclusive lock:

```python
import fcntl

def set(self, key, value):
    with self._lock():
        data = self._read_all()
        data[key] = value
        self._write_all(data)

def _lock(self):
    """Context manager for file locking."""
    # Acquire exclusive lock on storage file
    # Ensures atomic read-modify-write
```

**Pros:**
- Works across multiple processes
- Provides true mutual exclusion
- Relatively simple to implement

**Cons:**
- Platform-specific (fcntl on Unix, msvcrt on Windows)
- Adds a small performance overhead
- Requires careful handling of lock files

### Option 2: Atomic Operations

Redesign the storage layer to support atomic field updates:

```python
def set_atomic(self, key, field, value):
    """Atomically update a single field."""
    # Use database with ACID guarantees
    # Or implement log-structured updates
```

**Pros:**
- Best performance under high concurrency
- No lock contention

**Cons:**
- Major redesign of storage layer
- May not be feasible for file-based storage
- Breaks backward compatibility

### Option 3: In-Memory Locks (Not Recommended)

Use `threading.Lock()` or `multiprocessing.Lock()`:

**Cons:**
- Doesn't work across separate processes (only threads)
- Won't fix the issue for 4 separate API workers
- False sense of security

## Recommended Solution

**Use file-based locking (Option 1)** because:
1. Minimal code changes required
2. Works across processes (4 API workers)
3. Maintains backward compatibility
4. Acceptable performance overhead (<5ms per operation)
5. Can be implemented in a single PR

## Tradeoffs

### Performance Impact
- Adds lock acquisition/release overhead: ~1-5ms per operation
- Serializes concurrent writes (no parallel writes to same file)
- Acceptable tradeoff for data correctness

### Complexity
- Adds file lock management
- Need to handle lock cleanup on crashes
- Platform-specific code (Unix vs Windows)

### Backward Compatibility
- No API changes required
- Existing code continues to work
- Only internal locking added

## Verification

The fix is correct if:
1. The test from phase 2 passes (both concurrent writes persist)
2. No existing tests break
3. High concurrency testing shows no data loss
4. Performance degradation is minimal (<10ms)
