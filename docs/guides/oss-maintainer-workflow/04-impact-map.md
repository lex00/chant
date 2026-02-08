# Phase 4: Impact Map

Root cause analysis found the bug in `store.rs`. But before implementing a fix, you need to look wider. Does the same pattern exist elsewhere? What systems depend on the broken code? What tests and documentation need updating?

## Expanding the View

```bash
$ chant add "Impact map: issue #1234 concurrent write pattern"
Created spec: 2026-02-08-008-j3n
```

```yaml
---
type: research
labels: [impact-map, issue-1234]
depends_on: [005-k9w]
informed_by:
  - .chant/research/issue-1234-root-cause.md
target_files:
  - .chant/research/issue-1234-impact-map.md
---
```

```bash
$ chant work 008
Working 008-j3n: Impact map: issue #1234 concurrent write pattern
> Agent working in worktree /tmp/chant-008-j3n
...
Completed in 1m 50s
```

## What the Agent Finds

The agent searches the codebase for the same unprotected read-modify-write pattern:

```markdown
# Impact Map: Issue #1234

## Similar Patterns Found

| Location | Pattern | Affected? |
|----------|---------|-----------|
| src/storage/store.rs:145 | Read-modify-write without lock | YES - primary bug |
| src/storage/batch.rs:89 | Read-modify-write without lock | YES - same bug |
| src/cache/update.rs:203 | Read-modify-write with lock | NO - already safe |

## Dependent Systems

| System | File | Impact |
|--------|------|--------|
| CLI write command | src/cli/write.rs | Calls affected store.write() |
| Batch operations | src/storage/batch.rs | Has same unprotected pattern |
| API endpoint | src/api/write.rs | Calls affected store.write() |

## Test Coverage Gaps

Existing tests cover basic writes but no concurrency scenarios.
Tests to add:
- Concurrent write stress test
- Batch operation concurrency test
- Cross-process write test

## Documentation Updates Needed

- docs/architecture/storage.md - Update concurrency model
- CHANGELOG.md - Document the fix
```

The impact map found a second instance of the same bug in `batch.rs` and identified three test gaps. Without this phase, the fix would have addressed `store.rs` but left `batch.rs` vulnerable to the same race condition.

## Feeding the Implementation

The impact map produces the complete file list for the implementation spec:

```markdown
## Target Files for Implementation

Primary fixes:
- src/storage/store.rs - Add locking to write()
- src/storage/batch.rs - Add locking to batch operations

Tests:
- tests/storage/concurrent_test.rs - Add concurrency tests
- tests/regression/issue_1234_test.rs - Already exists from reproduction

Documentation:
- docs/architecture/storage.md - Update concurrency model
```

## When Impact Mapping Reveals Too Much

If the agent finds the pattern in 15 locations across 8 files, a single implementation spec is too large. Create a driver spec to coordinate multiple focused fixes:

```bash
$ chant add "Fix concurrent write bugs across storage layer"
Created spec: 2026-02-08-009-xyz

$ chant split 009
Analyzing spec 009...
Created 3 member specs:
  009.1 - Fix primary write path locking
  009.2 - Fix batch operation locking
  009.3 - Add comprehensive concurrency tests
```

For the kvstore bug, two affected locations is manageable in a single implementation spec.

**Next:** [Fork Fix](05-fork-fix.md)
