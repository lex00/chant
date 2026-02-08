# Phase 3: Root Cause

You have a failing test that proves the bug exists. Now you need to find out *why* it happens. This is the most demanding phase of the investigation -- and the most valuable. A thorough root cause analysis prevents fixing symptoms while the real bug remains.

## Starting Root Cause Research

```bash
$ chant add "Root cause: issue #1234 concurrent write data loss"
Created spec: 2026-02-08-005-k9w
```

The spec references everything learned so far:

```yaml
---
type: research
labels: [root-cause, issue-1234]
informed_by:
  - .chant/specs/2026-02-08-001-r4x.md
  - .chant/specs/2026-02-08-004-m2p.md
  - tests/regression/issue_1234_test.rs
  - src/storage/store.rs
  - src/storage/concurrent.rs
target_files:
  - .chant/research/issue-1234-root-cause.md
---
```

```bash
$ chant work 005
Working 005-k9w: Root cause: issue #1234 concurrent write data loss
> Agent working in worktree /tmp/chant-005-k9w
...
Completed in 3m 45s
```

## The Investigation

Most of the agent's time is spent forming, testing, and eliminating hypotheses. This loop -- hypothesize, test, record, iterate -- is where the real detective work happens. Each eliminated hypothesis narrows the search and documents what *doesn't* cause the issue, which is as valuable as finding what does.

The agent's root cause document captures this process:

```markdown
# Root Cause Analysis: Issue #1234

## Hypotheses Tested

| Hypothesis | Evidence | Result |
|------------|----------|--------|
| Filesystem cache coherency | Tested with direct I/O, disabled caching | Eliminated |
| Buffer overflow in write path | Buffer size checks, memory sanitizer | Eliminated |
| Lock timeout causing skip | Instrumented lock acquisition timing | Eliminated |
| Version counter race | Added logging for version assignment | Confirmed |
```

Four hypotheses tested, three eliminated. The version counter race is the culprit.

## The Root Cause

The agent traces the exact failure sequence:

```markdown
## What Happens

1. Thread A calls write("key", "value1")
2. Thread A reads current value, gets version 5
3. Thread B calls write("key", "value2")
4. Thread B reads current value, also gets version 5
5. Thread A writes with version 6
6. Thread B writes with version 6 (should be 7)
7. Thread B's write overwrites Thread A's without detection

## Why It Happens

The write() method in src/storage/store.rs:145 uses optimistic locking
that assumes writes are serialized at the filesystem level. This
assumption breaks under buffered I/O with concurrent writers.
```

The relevant code:

```rust
// src/storage/store.rs:145-150
fn write(&self, key: &str, value: &str) -> Result<()> {
    let current = self.read(key)?;      // Not locked
    let version = current.version + 1;
    // <-- Window where another write can interleave
    self.persist(key, value, version)    // May conflict
}
```

## Proposed Approaches

A good root cause analysis doesn't just find the bug -- it evaluates multiple fix strategies:

```markdown
## Approach 1: Pessimistic Locking
Acquire exclusive lock before read-modify-write.
+ Simple, uses existing Lock module
+ Guarantees correctness
- Reduces write throughput
Estimated: ~10 lines changed in store.rs

## Approach 2: Compare-and-Swap
Atomic CAS at the persistence layer.
+ Higher throughput, no deadlock risk
- More complex, needs retry logic
Estimated: ~50 lines across store.rs and persist.rs

## Approach 3: Write-Ahead Log
Log all writes, apply in order.
+ Preserves all writes, enables recovery
- Significant complexity, overkill for this use case
Estimated: ~200+ lines, new module

## Recommendation: Approach 1

Pessimistic locking is the simplest correct solution. It uses
the existing Lock module, and write throughput is not a critical
requirement per docs/architecture/storage.md. Can migrate to CAS
later if performance becomes an issue.
```

The recommendation is justified by the analysis, not by gut feeling. Future maintainers can read this document and understand why locking was chosen over CAS.

## When to Pivot

Investigation can hit dead ends. If you've tested several hypotheses and none are narrowing the search, step back:

- Re-read the issue thread with fresh eyes. Details you dismissed initially may be critical.
- Broaden the search to adjacent modules, callers, or dependencies.
- Verify the reproduction test actually matches the reported symptoms.
- Check environmental factors: configuration, platform differences, timing.

The goal is deliberate re-orientation when progress stalls, not endless persistence on an unproductive path.

## When Research Reveals Complexity

Sometimes root cause analysis shows the fix is bigger than expected. If the bug exists in multiple places or requires foundational changes first, split the implementation:

```bash
$ chant add "Refactor Lock module for reentrant locking"
Created spec: 2026-02-08-006-abc

$ chant add "Fix issue #1234 using reentrant locks"
Created spec: 2026-02-08-007-def
```

The second spec depends on the first. Chant's dependency tracking ensures they execute in order.

**Next:** [Impact Map](04-impact-map.md)
