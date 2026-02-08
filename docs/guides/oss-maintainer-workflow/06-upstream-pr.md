# Phase 6: Upstream PR

The staging PR in your fork looks good. CI passes, the fix is clean, the tests are comprehensive. Now a human -- you -- creates the real pull request to the upstream project.

## The Human Gate

This is the one phase the agent doesn't do. You review the staging PR, decide the timing is right, and create the upstream PR:

```bash
$ gh pr create \
  --repo upstream-org/kvstore \
  --base main \
  --head yourusername:fix/issue-1234 \
  --title "Fix #1234: Data loss on concurrent writes" \
  --body "$(cat <<'EOF'
## Summary

Fixes #1234. Concurrent writes to the same key could silently lose data
because the storage layer's optimistic locking didn't serialize the
read-modify-write cycle. Added pessimistic locking to both the single-write
and batch-write paths.

## Changes

- `src/storage/store.rs` -- Lock acquired before write operation
- `src/storage/batch.rs` -- Same fix for batch writes
- `tests/storage/concurrent_test.rs` -- Concurrency stress tests
- `tests/regression/issue_1234_test.rs` -- Regression test
- `docs/architecture/storage.md` -- Updated concurrency model

## Testing

All existing tests pass. Added regression test and concurrency stress tests.
EOF
)"
```

## Why a Human Gate?

The agent did the investigation and implementation, but humans make better decisions about:

- **Timing.** Don't submit during a code freeze or right before a release.
- **Communication.** Write the PR description in terms upstream maintainers understand, not in terms of your internal research process.
- **Scope.** Decide whether to bundle related fixes or submit them separately.
- **Relationship.** Maintain your standing with the upstream project.

## After Submission

Monitor the upstream PR. Address reviewer feedback by updating the staging PR first, then pushing to the upstream branch. If the upstream maintainers request a fundamentally different approach, that's a new research cycle.

## Archiving the Investigation

Once the upstream PR is merged, archive the specs:

```bash
$ chant archive 001
$ chant archive 004
$ chant archive 005
$ chant archive 008
$ chant archive 010
```

The investigation trail moves to `.chant/archive/` but remains available if a similar issue surfaces later.

**Next:** [Advanced Patterns](08-advanced.md)
