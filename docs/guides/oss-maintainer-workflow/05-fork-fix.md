# Phase 5: Fork Fix

Four phases of research have produced a clear picture: the bug, the fix strategy, and every file that needs to change. Now you implement.

## Creating the Implementation Spec

```bash
$ chant add "Fix issue #1234: add locking to concurrent writes"
Created spec: 2026-02-08-010-v7b
```

The spec references all research outputs:

```yaml
---
type: code
labels: [fix, issue-1234]
depends_on: [008-j3n]
informed_by:
  - .chant/research/issue-1234-root-cause.md
  - .chant/research/issue-1234-impact-map.md
target_files:
  - src/storage/store.rs
  - src/storage/batch.rs
  - tests/storage/concurrent_test.rs
  - docs/architecture/storage.md
---
```

```bash
$ chant work 010
Working 010-v7b: Fix issue #1234: add locking to concurrent writes
> Agent working in worktree /tmp/chant-010-v7b
...
Completed in 2m 30s
```

## What the Agent Implements

The agent reads the root cause analysis, which recommended pessimistic locking. It follows the recommendation:

```rust
fn write(&self, key: &str, value: &str) -> Result<()> {
    // Pessimistic lock prevents data loss during concurrent writes.
    // See: .chant/research/issue-1234-root-cause.md
    let _guard = self.lock.acquire(key)?;
    let current = self.read(key)?;
    let version = current.version + 1;
    self.persist(key, value, version)
}
```

The same fix goes into `batch.rs`, the second location identified by the impact map. The agent also adds concurrency tests beyond the original reproduction test, covering edge cases the research identified: lock timeouts, partial failures, and cross-process writes.

The reproduction test now passes:

```
running 3 tests
test regression::issue_1234_concurrent_write_loses_data ... ok
test storage::concurrent_write_stress ... ok
test storage::concurrent_batch_write ... ok
```

## Why Fork-Internal Staging?

For open source work, the agent's changes land in your fork, not upstream. You create a staging PR within your fork (`yourfork:fix/issue-1234` to `yourfork:main`) to review the changes before exposing them upstream:

```bash
$ gh pr create \
  --repo yourusername/kvstore \
  --base main \
  --head fix/issue-1234 \
  --title "Fix #1234: Data loss on concurrent writes" \
  --body "$(cat <<'EOF'
## Summary
Research-backed fix using pessimistic locking for concurrent write safety.

## Changes
- Added locking to write path in store.rs and batch.rs
- Added concurrency stress tests
- Updated architecture documentation

## Testing
- Regression test passes
- New concurrency tests pass
- All existing tests pass
EOF
)"
```

This staging PR lets you:
- Run CI in your fork before going upstream
- Iterate on the fix without upstream visibility
- Review agent output before it becomes a public contribution

## Following the Research

The implementation should match what the research recommended. If the agent discovers the recommended approach won't work during implementation -- say, the Lock module doesn't support reentrant locking for nested writes -- the right response is to stop, document the finding, and create a new research spec:

```bash
$ chant add "Re-research #1234: pessimistic locking insufficient"
Created spec: 2026-02-08-011-abc
```

Don't improvise a different approach mid-implementation. The research exists for a reason.

## Keeping Changes Focused

The diff should contain only what the research identified. No unrelated refactoring, no "while I'm here" improvements. A focused PR is easier to review, easier to revert if needed, and easier for upstream maintainers to understand.

**Next:** [Upstream PR](06-upstream-pr.md)
