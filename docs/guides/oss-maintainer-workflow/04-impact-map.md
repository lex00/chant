# Impact Map Research

Expand the view beyond the immediate root cause to identify all affected areas.

## Why Impact Mapping?

After identifying the root cause, you need to expand your understanding:

- **Related code** that uses similar patterns
- **Dependent systems** that might be affected
- **Test coverage** that needs updating
- **Documentation** that needs changes
- **Similar bugs** elsewhere in the codebase

An impact map research spec takes the root cause findings and expands the view to identify all files that need modification.

## Impact Mapping Workflow

```
Root Cause      Impact Map         Expanded View      Implementation
  Output         Research               │                 Input
    │                 │                 ▼                   │
    ▼                 ▼           ┌──────────┐              ▼
┌─────────┐     ┌───────────┐    │ All      │        ┌───────────┐
│ Primary │────▶│ Expand    │───▶│ affected │───────▶│ informed  │
│ bug     │     │ from root │    │ files    │        │ by: all   │
│ location│     │ cause     │    └──────────┘        │ research  │
└─────────┘     └───────────┘                        └───────────┘
```

## Creating an Impact Map Research Spec

```bash
chant add "Impact Map: issue #1234"
```

Edit the spec to reference root cause research:

```yaml
---
type: research
status: ready
depends_on:
  - 003-root-cause
target_files:
  - .chant/research/issue-1234-impact-map.md
prompt: research
informed_by:
  - .chant/research/issue-1234-root-cause.md
---

# Phase 4: Impact Map - Assess impact of Issue #1234 bug pattern

## Context

Phase 3 identified the root cause: [brief summary of the bug]. Before implementing a fix, we need to understand:
- Is this pattern used elsewhere in the codebase?
- What other systems might be affected?
- How big is this fix going to be?

## Research Questions

- [ ] What other code uses the same pattern?
- [ ] What tests need to be updated or added?
- [ ] What documentation references this behavior?
- [ ] Are there similar bugs elsewhere?
- [ ] What edge cases need consideration?

## Acceptance Criteria

- [ ] All code using similar patterns identified
- [ ] Test files that need updates listed
- [ ] Documentation that needs updates listed
- [ ] Complete file list for implementation phase
- [ ] Edge cases and risks documented
```

## Impact Map Output

The spec produces an impact map document with comprehensive target files:

```markdown
# Impact Map Research: Issue #1234

**Date:** 2026-02-02
**Informed by:** Root cause research (2026-02-02-003-ghi)

## Root Cause Summary

Bug located in `src/storage/store.rs:145` where optimistic locking
fails under concurrent writes.

## Impact Analysis

### Similar Patterns Found

| Location | Pattern | Affected? |
|----------|---------|-----------|
| `src/storage/store.rs:145` | Read-modify-write without lock | **YES** - Primary bug |
| `src/storage/batch.rs:89` | Read-modify-write without lock | **YES** - Same bug |
| `src/cache/update.rs:203` | Read-modify-write with lock | **NO** - Already safe |

### Dependent Systems

| System | File | Impact |
|--------|------|--------|
| CLI write command | `src/cli/write.rs` | Calls affected store.write() |
| Batch operations | `src/storage/batch.rs` | Has same bug, needs fix |
| API endpoint | `src/api/write.rs` | Calls affected store.write() |

### Test Coverage

**Existing tests:**
- `tests/storage/basic_test.rs` - Basic write tests (no concurrency)
- `tests/storage/concurrent_test.rs` - Has tests but incomplete

**Tests to add:**
- Concurrent write stress test
- Batch operation concurrency test
- Cross-process write test

**Test files needing updates:**
- `tests/storage/concurrent_test.rs` - Add comprehensive tests
- `tests/cli/write_test.rs` - Add concurrent CLI test

### Documentation

**Documentation needing updates:**
- `docs/architecture/storage.md` - Update concurrency model
- `README.md` - Note about concurrent write safety
- `CHANGELOG.md` - Document breaking change if any

## Target Files for Implementation

Implementation spec should modify:

**Primary fixes:**
- `src/storage/store.rs` - Add locking to write()
- `src/storage/batch.rs` - Add locking to batch operations

**Tests:**
- `tests/storage/concurrent_test.rs` - Add comprehensive tests
- `tests/regression/issue_1234_test.rs` - Already exists from repro

**Documentation:**
- `docs/architecture/storage.md` - Update concurrency model

## Edge Cases

1. **Nested writes:** What if write() calls write()?
   - Need reentrant locking or document limitation

2. **Batch atomicity:** Should batch operations be atomic?
   - Current: No atomicity
   - Proposed: Add transaction support

3. **Cross-process locking:** CLI invocations are separate processes
   - Need file-based locking mechanism

## Recommendations for Implementation

1. **Phase 1:** Fix primary bug in store.rs
2. **Phase 2:** Fix similar bug in batch.rs
3. **Phase 3:** Add comprehensive tests
4. **Phase 4:** Update documentation

Alternative: Create separate specs for each phase if complex.
```

## Using informed_by Chain

Impact map research is informed by root cause research:

```yaml
# Root cause research (003) finds the bug
type: research
depends_on:
  - 002-reproduction
informed_by:
  - .chant/research/issue-1234-comprehension.md
target_files:
  - .chant/research/issue-1234-root-cause.md

# Impact map research (004) expands the view
type: research
depends_on:
  - 003-root-cause
informed_by:
  - .chant/research/issue-1234-root-cause.md
target_files:
  - .chant/research/issue-1234-impact-map.md

# Implementation (005) uses both research outputs
type: code
depends_on:
  - 004-impact-map
informed_by:
  - .chant/research/issue-1234-root-cause.md
  - .chant/research/issue-1234-impact-map.md
```

## When Impact Mapping Reveals Complexity

Sometimes impact map research reveals the fix is more complex than expected:

```markdown
## Impact Findings

Analysis revealed 15 locations with the same pattern across 8 files.
Fixing all instances is too large for a single implementation spec.

## Recommendation

Create a driver spec to coordinate fixes:

1. **Core fix:** Fix primary bug location
2. **Similar bugs:** Fix 14 other locations
3. **Tests:** Add comprehensive test coverage
4. **Documentation:** Update architecture docs

Each phase should be a separate spec with dependencies.
```

## See Also

- [Root Cause Research](03-root-cause.md) — Previous step: find the bug
- [Fork Fix](05-fork-fix.md) — Next step: implement the fix
