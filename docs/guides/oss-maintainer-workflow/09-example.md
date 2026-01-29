# Complete Walkthrough

A real-world example from issue report to merged fix.

## The Issue

A user reports data loss in your storage library:

```markdown
# GitHub Issue #1234

**Title:** Data lost when saving from multiple terminals

**Description:**
I'm using `yourproject` to manage configuration files. When I edit
config from two terminal windows simultaneously, sometimes one edit
disappears completely.

**Steps to reproduce:**
1. Open two terminals
2. In terminal 1: `yourproject write config.key "value1"`
3. In terminal 2: `yourproject write config.key "value2"`
4. Run quickly so they overlap
5. Check result: sometimes neither value is saved correctly

**Expected:** One of the values should win (last write wins)
**Actual:** Sometimes get corrupted data like "valu" or empty

**Environment:**
- macOS 14.2
- yourproject 0.5.2
- Built from source with Rust 1.75
```

## Phase 1: Triage

### Create Triage Spec

```bash
chant add "Triage issue #1234: Data lost on concurrent writes" --type task
```

Edit `.chant/specs/2026-01-29-001-abc.md`:

```yaml
---
type: task
status: ready
prompt: triage
labels:
  - triage
  - issue-1234
target_files:
  - .chant/triage/issue-1234.md
---

# Triage issue #1234: Data lost on concurrent writes

## Issue Summary

User reports data loss/corruption when writing to same key from multiple
terminals simultaneously. Describes truncated data ("valu" instead of
"value1") and empty results.

## Acceptance Criteria

- [ ] Issue categorized
- [ ] Priority assigned
- [ ] Missing information identified
- [ ] Recommendation provided
```

### Execute Triage

```bash
$ chant work 2026-01-29-001-abc --prompt triage

Working spec: 2026-01-29-001-abc
Agent analyzing issue...

✓ Spec completed
```

### Triage Output

`.chant/triage/issue-1234.md`:

```markdown
# Triage: Issue #1234

**Date:** 2026-01-29
**Triaged by:** chant agent

## Assessment

| Field | Value |
|-------|-------|
| Category | Bug |
| Priority | High |
| Severity | Data loss |
| Completeness | Good |

## Analysis

The report describes a classic concurrent write race condition. Data
corruption ("valu") suggests writes are interleaving at the byte level,
not at the record level. This is a serious data integrity issue.

The reproduction steps are clear and the environment details are complete.

## Recommendation

**needs-reproduction**

Create a failing test that demonstrates the bug before investigating
root cause. The symptoms suggest a synchronization issue in the write
path.

## Questions for User

None needed - report is complete enough to proceed.
```

## Phase 2: Reproduction

### Create Reproduction Spec

```bash
chant add "Reproduce issue #1234: Concurrent write data loss" --type task
```

Edit `.chant/specs/2026-01-29-002-def.md`:

```yaml
---
type: task
status: ready
prompt: reproduce
labels:
  - reproduction
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-001-abc.md
  - .chant/triage/issue-1234.md
target_files:
  - tests/regression/issue_1234_test.rs
---

# Reproduce issue #1234: Concurrent write data loss

## Context

Triage confirmed this is a bug with clear reproduction steps.
Need to create a failing test before research.

## Acceptance Criteria

- [ ] Minimal test case demonstrates the bug
- [ ] Test fails consistently
- [ ] Test is self-contained
- [ ] Environment documented in comments
```

### Execute Reproduction

```bash
$ chant work 2026-01-29-002-def --prompt reproduce

Working spec: 2026-01-29-002-def
Agent creating reproduction test...

✓ Spec completed
```

### Reproduction Output

`tests/regression/issue_1234_test.rs`:

```rust
//! Regression test for issue #1234: Concurrent write data loss
//!
//! Issue: https://github.com/yourproject/issues/1234
//! Reporter: @username
//! Environment: macOS 14.2, version 0.5.2

use std::sync::Arc;
use std::thread;
use yourproject::Store;

#[test]
fn issue_1234_concurrent_write_causes_data_loss() {
    // Stress test to catch race condition
    for iteration in 0..100 {
        let store = Arc::new(Store::new_temp().unwrap());

        // Write initial value
        store.write("key", "initial").unwrap();

        // Spawn concurrent writers
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let store = Arc::clone(&store);
                let value = format!("value{}", i);
                thread::spawn(move || {
                    store.write("key", &value).unwrap();
                })
            })
            .collect();

        // Wait for all writes
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify data integrity
        let result = store.read("key").unwrap();

        // Bug: Sometimes get corrupted data
        assert!(
            result.starts_with("value") || result == "initial",
            "Iteration {}: Data corrupted: '{}'",
            iteration,
            result
        );

        // Bug: Sometimes data is truncated
        assert!(
            result.len() >= 6,
            "Iteration {}: Data truncated: '{}' (len={})",
            iteration,
            result,
            result.len()
        );
    }
}
```

### Verify Test Fails

```bash
$ cargo test issue_1234

running 1 test
test regression::issue_1234_concurrent_write_causes_data_loss ... FAILED

failures:

---- regression::issue_1234_concurrent_write_causes_data_loss stdout ----
thread 'regression::issue_1234_concurrent_write_causes_data_loss' panicked at
tests/regression/issue_1234_test.rs:47:9:
Iteration 23: Data corrupted: 'valu'

failures:
    regression::issue_1234_concurrent_write_causes_data_loss

test result: FAILED. 0 passed; 1 failed
```

Bug confirmed and reproducible.

## Phase 3: Research

### Create Research Spec

```bash
chant add "Research root cause: issue #1234 concurrent write data loss" --type task
```

Edit `.chant/specs/2026-01-29-003-ghi.md`:

```yaml
---
type: task
status: ready
prompt: research
labels:
  - research
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-002-def.md
  - tests/regression/issue_1234_test.rs
  - docs/architecture/storage.md
  - src/storage/store.rs
  - src/storage/persist.rs
target_files:
  - .chant/research/issue-1234-rca.md
---

# Research root cause: issue #1234 concurrent write data loss

## Context

Reproduction test confirms data corruption during concurrent writes.
Need to understand why before fixing.

## Research Questions

- [ ] Where exactly does corruption occur?
- [ ] What synchronization exists currently?
- [ ] Why does it fail under concurrency?
- [ ] What are the fix options?

## Acceptance Criteria

- [ ] Root cause identified with code references
- [ ] Multiple approaches evaluated
- [ ] Recommended approach justified
- [ ] Edge cases documented
```

### Execute Research

```bash
$ chant work 2026-01-29-003-ghi --prompt research

Working spec: 2026-01-29-003-ghi
Agent investigating codebase...

✓ Spec completed
```

### Research Output

`.chant/research/issue-1234-rca.md`:

```markdown
# Root Cause Analysis: Issue #1234

**Date:** 2026-01-29
**Spec:** 2026-01-29-003-ghi

## Executive Summary

Data corruption occurs because `write()` uses non-atomic file operations.
Multiple concurrent writes can interleave at the filesystem level,
producing partial writes.

## Root Cause

### The Bug

In `src/storage/persist.rs:78`:

```rust
pub fn persist(&self, key: &str, value: &str) -> Result<()> {
    let path = self.path_for(key);
    let mut file = File::create(&path)?;  // Truncates file
    file.write_all(value.as_bytes())?;     // Write may interleave
    Ok(())
}
```

The sequence `create` → `write_all` is not atomic:

1. Thread A: `File::create()` truncates file to 0 bytes
2. Thread B: `File::create()` truncates file again
3. Thread A: `write_all("value1")` writes "value1"
4. Thread B: `write_all("value2")` writes "value2" starting at offset 0
5. Result: File may contain "value2" or "valu2" depending on timing

### Why It Happens

The code assumes single-threaded access. The `File::create()` call
truncates before writing, creating a window where another writer can
also truncate, then both write to the same file descriptor.

## Potential Approaches

### Approach 1: File Locking

**Description:** Use `flock()` or platform equivalent before write.

```rust
pub fn persist(&self, key: &str, value: &str) -> Result<()> {
    let path = self.path_for(key);
    let file = File::create(&path)?;
    file.lock_exclusive()?;  // Block other writers
    file.write_all(value.as_bytes())?;
    // Lock released on drop
    Ok(())
}
```

**Pros:** Simple, uses OS primitives
**Cons:** Platform differences, advisory locks only

### Approach 2: Write-then-Rename

**Description:** Write to temp file, atomically rename.

```rust
pub fn persist(&self, key: &str, value: &str) -> Result<()> {
    let path = self.path_for(key);
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, value)?;
    std::fs::rename(&temp, &path)?;  // Atomic on most filesystems
    Ok(())
}
```

**Pros:** Atomic on POSIX, no partial writes possible
**Cons:** Extra disk I/O, temp file cleanup needed

### Approach 3: Application-Level Locking

**Description:** Use in-memory lock before file operations.

```rust
pub fn persist(&self, key: &str, value: &str) -> Result<()> {
    let _guard = self.locks.lock(key);  // Per-key lock
    let path = self.path_for(key);
    std::fs::write(&path, value)?;
    Ok(())
}
```

**Pros:** Fast, works across platforms
**Cons:** Doesn't protect cross-process writes

## Recommendation

**Approach 2: Write-then-Rename**

**Justification:**
1. Guarantees atomic writes even across processes
2. No platform-specific locking code
3. Also protects against crashes (no partial files)
4. Industry standard pattern (SQLite, etcd, etc.)

Combined with Approach 3 for in-process optimization:
- Use per-key in-memory lock for performance
- Write-then-rename for actual persistence
- Both layers for defense in depth

## Files to Modify

1. `src/storage/persist.rs` - Implement atomic write
2. `src/storage/store.rs` - Add per-key locking
3. `tests/regression/issue_1234_test.rs` - Verify fix

## Edge Cases

1. **Temp file cleanup:** Handle crash during write
2. **Disk full:** Detect before rename
3. **Cross-filesystem:** Rename may not be atomic
4. **Permissions:** Temp file needs same perms as target
```

## Phase 4: Implementation

### Create Implementation Spec

```bash
chant add "Fix issue #1234: Atomic writes for concurrent access" --type code
```

Edit `.chant/specs/2026-01-29-004-jkl.md`:

```yaml
---
type: code
status: ready
prompt: implement
labels:
  - fix
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-003-ghi.md
  - .chant/research/issue-1234-rca.md
target_files:
  - src/storage/persist.rs
  - src/storage/store.rs
  - tests/regression/issue_1234_test.rs
---

# Fix issue #1234: Atomic writes for concurrent access

## Context

Research recommends write-then-rename with per-key locking.
See `.chant/research/issue-1234-rca.md`.

## Acceptance Criteria

- [ ] Atomic write-then-rename implemented
- [ ] Per-key in-memory locking added
- [ ] Reproduction test passes
- [ ] Edge cases handled (cleanup, permissions)
- [ ] All existing tests pass
- [ ] Release notes added

## Release Notes

<!-- To be filled after implementation -->
```

### Execute Implementation

```bash
$ chant work 2026-01-29-004-jkl --prompt implement

Working spec: 2026-01-29-004-jkl
Agent implementing fix...

✓ Spec completed
```

### Implementation Changes

`src/storage/persist.rs`:

```rust
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

impl Persister {
    /// Atomically persist a value using write-then-rename pattern.
    ///
    /// This ensures concurrent writes cannot corrupt data, even across
    /// processes. See issue #1234 for details.
    pub fn persist(&self, key: &str, value: &str) -> Result<()> {
        let path = self.path_for(key);
        let temp = self.temp_path_for(key);

        // Write to temporary file
        {
            let mut file = File::create(&temp)?;
            file.write_all(value.as_bytes())?;
            file.sync_all()?;  // Ensure written to disk
        }

        // Copy permissions from existing file or parent dir
        self.copy_permissions(&path, &temp)?;

        // Atomic rename
        fs::rename(&temp, &path)?;

        Ok(())
    }

    fn temp_path_for(&self, key: &str) -> PathBuf {
        let path = self.path_for(key);
        path.with_extension(format!("tmp.{}", std::process::id()))
    }

    fn copy_permissions(&self, target: &Path, temp: &Path) -> Result<()> {
        // ... permission handling
    }
}
```

`src/storage/store.rs`:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct Store {
    persister: Persister,
    locks: KeyLocks,
}

/// Per-key locks for in-process synchronization.
struct KeyLocks {
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl Store {
    pub fn write(&self, key: &str, value: &str) -> Result<()> {
        // Acquire per-key lock (in-process synchronization)
        let _guard = self.locks.acquire(key);

        // Atomic write (cross-process safety)
        self.persister.persist(key, value)
    }
}
```

### Verify Fix

```bash
$ cargo test issue_1234

running 1 test
test regression::issue_1234_concurrent_write_causes_data_loss ... ok

test result: ok. 1 passed; 0 failed
```

### Update Release Notes in Spec

```yaml
## Release Notes

### Fixed: Data corruption during concurrent writes (#1234)

Previously, writing to the same key from multiple processes or threads
could cause data corruption or loss. This occurred because writes were
not atomic at the filesystem level.

**Impact:** Users who experienced data corruption during concurrent CLI
invocations or multi-threaded access should see reliable behavior after
upgrading.

**Technical details:** Writes now use the atomic write-then-rename pattern,
combined with per-key in-memory locking. This ensures data integrity both
within a process and across multiple processes.
```

## Phase 5: Review

### Create Review Spec

```bash
chant add "Review fix for issue #1234" --type task
```

Edit `.chant/specs/2026-01-29-005-mno.md`:

```yaml
---
type: task
status: ready
prompt: review
labels:
  - review
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-002-def.md
  - .chant/specs/2026-01-29-004-jkl.md
  - tests/regression/issue_1234_test.rs
target_files:
  - .chant/reviews/issue-1234-review.md
---

# Review fix for issue #1234

## Acceptance Criteria

- [ ] Reproduction test passes
- [ ] All tests pass
- [ ] Code follows research recommendation
- [ ] Edge cases handled
- [ ] Documentation adequate
```

### Execute Review

```bash
$ chant work 2026-01-29-005-mno --prompt review

Working spec: 2026-01-29-005-mno
Agent reviewing implementation...

✓ Spec completed
```

### Review Output

```markdown
# Review: Issue #1234 Fix

**Verdict: APPROVED**

## Checklist

| Check | Result |
|-------|--------|
| Reproduction test | ✅ Pass |
| All tests | ✅ 248 pass, 0 fail |
| Matches research | ✅ Write-then-rename + per-key locks |
| Edge cases | ✅ Permissions, cleanup handled |

## Notes

Implementation follows research recommendation exactly. Atomic write
pattern is correctly implemented. Temp file cleanup handles process
ID to avoid conflicts.

Ready for merge.
```

### Approve and Merge

```bash
$ chant approve 2026-01-29-004-jkl --by "maintainer-name"

Approved: 2026-01-29-004-jkl

$ chant merge --all

Merging 5 specs...
✓ 2026-01-29-001-abc (triage)
✓ 2026-01-29-002-def (reproduction)
✓ 2026-01-29-003-ghi (research)
✓ 2026-01-29-004-jkl (implementation)
✓ 2026-01-29-005-mno (review)

All specs merged.
```

## Phase 6: Documentation (Optional)

If docs need updating:

```bash
chant add "Document atomic write behavior" --type documentation
# ... work the spec
```

## Phase 7: Release

```bash
chant add "Release notes for v0.6.1" --type task

# Work release spec to update CHANGELOG
chant work <release-spec-id>

# Archive completed specs
chant archive --label issue-1234
```

## Summary

Total workflow:

| Phase | Spec | Time | Output |
|-------|------|------|--------|
| Triage | 001-abc | ~2 min | Assessment + recommendation |
| Reproduction | 002-def | ~5 min | Failing test |
| Research | 003-ghi | ~10 min | Root cause analysis |
| Implementation | 004-jkl | ~15 min | Working fix |
| Review | 005-mno | ~5 min | Approval |

The research-first approach took longer than a quick hack, but produced:
- Auditable decision trail
- Comprehensive test coverage
- Correct fix addressing root cause
- Documentation for future maintainers

## See Also

- [Index](index.md) — Overview and quick start
- [Advanced Patterns](08-advanced.md) — More complex scenarios
