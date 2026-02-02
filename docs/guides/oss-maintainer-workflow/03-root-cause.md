# Root Cause Research

Determine what needs to be fixed by investigating WHY the bug exists.

## Why Research First?

The most common mistake in bug fixing is jumping straight to code changes. Without understanding the root cause:

- You might fix a symptom while the real bug remains
- Your fix might introduce regressions in related code
- You miss opportunities to fix similar bugs elsewhere
- Future maintainers won't understand why the code changed

A research spec produces a comprehensive analysis that informs the implementation.

## Research Workflow

```
Reproduction      Research Spec         RCA Document        Implementation
   Output              │                     │                  Input
     │                 ▼                     ▼                    │
     ▼           ┌───────────┐        ┌───────────────┐           ▼
┌──────────┐     │ Run       │        │ • Root cause  │     ┌───────────┐
│ Failing  │────▶│ research  │───────▶│ • Approaches  │────▶│ informed  │
│ test     │     │ prompt    │        │ • Trade-offs  │     │ by: RCA   │
└──────────┘     └───────────┘        │ • Recommend   │     └───────────┘
     │                                └───────────────┘
     │
     └── Also: relevant docs, source files
```

## Creating a Root Cause Research Spec

```bash
chant add "Root cause: issue #1234 concurrent write data loss" --type research
```

Edit the spec to reference comprehension output:

```yaml
---
type: research
status: ready
labels:
  - root-cause
  - issue-1234
informed_by:
  - .chant/specs/2026-02-02-001-abc.md    # Comprehension spec
  - .chant/specs/2026-02-02-002-def.md    # Reproduction spec
  - tests/regression/issue_1234_test.rs    # The failing test
target_files:
  - .chant/research/issue-1234-root-cause.md
---

# Root cause: issue #1234 concurrent write data loss

## Context

Comprehension research identified storage layer as affected component.
Reproduction spec confirmed data loss during concurrent writes.
Failing test: `tests/regression/issue_1234_test.rs`

## Research Questions

- [ ] Where exactly does data loss occur in the write path?
- [ ] What synchronization mechanism is currently used?
- [ ] Why does the synchronization fail under concurrency?

## Acceptance Criteria

- [ ] Root cause identified with code references
- [ ] Affected code paths documented
- [ ] Target files list produced for sprawl research phase
```

## The Research Prompt

The `research` prompt instructs the agent to investigate thoroughly:

```markdown
You are conducting deep root cause analysis for an issue.

Your goal is to:
1. Thoroughly explore the codebase to understand WHY the issue exists
2. Trace code paths, identify affected components
3. Read relevant local documentation and source files
4. Consider historical context (git blame, related commits)
5. Identify potential approaches and their trade-offs

Instructions:
- Read extensively before forming conclusions
- Use informed_by to reference reproduction specs, docs, source files
- Be thorough - this analysis will guide implementation
- Document your reasoning clearly for future implementers

Output:
- Comprehensive root cause analysis
- Affected components and why
- 2-3 potential approaches with trade-offs
- Recommended approach with justification
- Files that need modification
- Edge cases to consider
```

## Using `informed_by` for Research

The `informed_by` field is critical for research specs. It tells the agent what context to consider:

### Referencing Prior Specs

```yaml
informed_by:
  - .chant/specs/2026-01-29-002-def.md  # Reproduction spec
```

The agent reads the reproduction spec to understand:
- What the failing test demonstrates
- What environment details were captured
- Any observations from reproduction attempts

### Referencing Documentation

```yaml
informed_by:
  - docs/architecture/storage.md
  - docs/design/concurrency-model.md
```

The agent reads architecture docs to understand:
- Intended design patterns
- Assumptions the code makes
- Why certain approaches were chosen

### Referencing Source Files

```yaml
informed_by:
  - src/storage/store.rs
  - src/storage/concurrent.rs
  - src/storage/lock.rs
```

The agent reads source files to:
- Trace the actual implementation
- Identify where bugs might occur
- Understand the current synchronization approach

### Using Globs

```yaml
informed_by:
  - src/storage/**/*.rs
```

For broader investigation across a module.

## Research Output Structure

A comprehensive RCA document includes:

```markdown
# Root Cause Analysis: Issue #1234

**Date:** 2026-01-29
**Analyzed by:** chant agent
**Spec:** 2026-01-29-003-ghi

## Executive Summary

Data loss occurs because the `write()` method in `store.rs` uses
optimistic locking that doesn't handle the case where two writes
complete their read phase before either starts writing.

## Root Cause

### What Happens

1. Thread A calls `write("key", "value1")`
2. Thread A reads current value, gets version 5
3. Thread B calls `write("key", "value2")`
4. Thread B reads current value, gets version 5
5. Thread A writes with version 6
6. Thread B writes with version 6 (should be 7)
7. Thread B's write overwrites Thread A's without merging

### Why It Happens

The optimistic locking implementation in `src/storage/store.rs:145`
assumes writes are serialized at the filesystem level, but this
assumption doesn't hold when using buffered I/O.

Relevant code:

```rust
// src/storage/store.rs:145-160
fn write(&self, key: &str, value: &str) -> Result<()> {
    let current = self.read(key)?;  // Read is not locked
    let version = current.version + 1;
    // Window here where another write can occur
    self.persist(key, value, version)  // Write may conflict
}
```

### Historical Context

`git blame` shows this code was added in commit `abc123` (2024-03-15)
during the initial storage implementation. The commit message mentions
"optimistic locking for performance" but doesn't address concurrent
write scenarios.

## Affected Components

| Component | File | Impact |
|-----------|------|--------|
| Store write path | `src/storage/store.rs` | Primary bug location |
| Concurrent module | `src/storage/concurrent.rs` | Unused, could provide fix |
| Lock module | `src/storage/lock.rs` | Exists but not used for writes |
| CLI write command | `src/cli/write.rs` | Calls affected store method |

## Potential Approaches

### Approach 1: Pessimistic Locking

**Description:** Acquire exclusive lock before read-modify-write cycle.

**Pros:**
- Simple to implement
- Guarantees correctness
- Uses existing `Lock` module

**Cons:**
- Reduces write throughput
- Potential for deadlocks if not careful
- Blocks reads during writes

**Implementation:**
```rust
fn write(&self, key: &str, value: &str) -> Result<()> {
    let _guard = self.lock.acquire(key)?;
    let current = self.read(key)?;
    let version = current.version + 1;
    self.persist(key, value, version)
}
```

**Estimated changes:** ~10 lines in `store.rs`

### Approach 2: Compare-and-Swap

**Description:** Use atomic compare-and-swap at persistence layer.

**Pros:**
- Higher throughput than pessimistic locking
- No deadlock risk
- Better for read-heavy workloads

**Cons:**
- More complex implementation
- Requires retry logic
- May need filesystem-specific code

**Implementation:**
```rust
fn write(&self, key: &str, value: &str) -> Result<()> {
    loop {
        let current = self.read(key)?;
        let version = current.version + 1;
        match self.cas_persist(key, value, current.version, version) {
            Ok(()) => return Ok(()),
            Err(VersionMismatch) => continue,  // Retry
            Err(e) => return Err(e),
        }
    }
}
```

**Estimated changes:** ~50 lines across `store.rs` and `persist.rs`

### Approach 3: Write-Ahead Log

**Description:** Log all writes, apply in order.

**Pros:**
- Preserves all writes (no lost updates)
- Enables recovery
- Better audit trail

**Cons:**
- Significant complexity
- Performance overhead
- Overkill for current requirements

**Estimated changes:** ~200+ lines, new module

## Recommendation

**Approach 1: Pessimistic Locking**

**Justification:**
1. Simplest correct solution
2. Uses existing `Lock` module (tested and proven)
3. Write throughput isn't a critical requirement per `docs/architecture/storage.md`
4. Can migrate to CAS later if performance becomes an issue

## Edge Cases to Consider

1. **Lock timeout:** What if lock acquisition times out?
   - Recommendation: Return error, let caller retry

2. **Nested writes:** What if write triggers another write?
   - Recommendation: Use reentrant locks

3. **Partial failures:** What if write succeeds but unlock fails?
   - Recommendation: Use RAII guard pattern

4. **Cross-process writes:** CLI invocations are separate processes
   - Recommendation: File-based locking (already supported by `Lock`)

## Files to Modify

1. `src/storage/store.rs` - Add lock acquisition to write path
2. `tests/regression/issue_1234_test.rs` - Verify fix
3. `tests/storage/concurrent_test.rs` - Add comprehensive concurrency tests
4. `docs/architecture/storage.md` - Update to document locking behavior

## Risks

1. **Performance regression:** Monitor write latency after fix
2. **Deadlock potential:** Review all call sites for lock ordering
3. **Breaking change:** External code calling `write()` now blocks
```

## Research Anti-Patterns

### Insufficient Investigation

❌ **Bad:**
```markdown
## Root Cause
The write function doesn't handle concurrency.

## Recommendation
Add locking.
```

✅ **Good:**
[Full analysis as shown above]

### Premature Solution

❌ **Bad:**
```markdown
## Root Cause
Haven't fully investigated yet.

## Recommendation
Let's try adding a mutex and see if it works.
```

✅ **Good:**
Complete the research questions before recommending solutions.

### Missing Trade-offs

❌ **Bad:**
```markdown
## Potential Approaches
Use pessimistic locking. It's the best approach.
```

✅ **Good:**
Present multiple approaches with honest trade-offs, then justify selection.

## Spec Completion

When research is complete:

```yaml
---
type: task
status: completed
prompt: research
labels:
  - research
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-002-def.md
  - tests/regression/issue_1234_test.rs
  - docs/architecture/storage.md
  - src/storage/store.rs
target_files:
  - .chant/research/issue-1234-rca.md
model: claude-sonnet-4-20250514
completed_at: 2026-01-29T16:00:00Z
---
```

The completed research spec becomes the primary input for implementation.

## When Research Reveals Complexity

Sometimes research reveals the fix is more complex than expected:

### Split into Phases

```bash
# Phase 1: Foundational changes
chant add "Refactor Lock module for reentrant locking" --type code

# Phase 2: The actual fix (depends on phase 1)
chant add "Fix issue #1234 using reentrant locks" --type code
# Add: depends_on: [phase-1-spec-id]
```

### Expand Scope

If research reveals the bug exists in multiple places:

```bash
# Create a driver spec to coordinate fixes
chant add "Fix concurrent write bugs across storage layer" --type driver
# The driver creates member specs for each location
```

## See Also

- [Reproduction Case](02-reproduction.md) — Previous step: confirm the bug exists
- [Codebase Sprawl Research](04-sprawl.md) — Next step: expand investigation scope
