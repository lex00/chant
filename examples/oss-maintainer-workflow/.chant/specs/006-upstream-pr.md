---
type: documentation
status: pending
depends_on:
- 005-fork-fix
informed_by:
- .chant/research/issue-42-comprehension.md
- .chant/research/issue-42-root-cause.md
- .chant/research/issue-42-sprawl.md
---
# Phase 6: Upstream PR - Human gate before opening real PR

## Context

The fix has been implemented and tested in our fork. Before creating an upstream PR to the main project, we need a human review to ensure:
- The fix is correct and complete
- We're not introducing new issues
- The PR message explains the problem and solution clearly
- Timing is appropriate for the project

## Human Review Checklist

### Technical Verification

- [ ] All tests pass locally, including the new regression test
- [ ] Fix has been tested under high concurrency
- [ ] No performance degradation measured
- [ ] Code follows project style guidelines
- [ ] Backward compatibility maintained
- [ ] No unintended side effects observed

### Documentation

- [ ] Code includes clear comments explaining the fix
- [ ] Commit message follows project conventions
- [ ] PR description explains the problem and solution
- [ ] Links to issue #42
- [ ] Includes before/after behavior description

### Research Artifacts

Review the research documents to inform the PR description:
- `.chant/research/issue-42-comprehension.md` - Problem understanding
- `.chant/research/issue-42-root-cause.md` - Technical explanation
- `.chant/research/issue-42-sprawl.md` - Scope and impact

### PR Description Template

```markdown
## Summary
Fixes #42 - Concurrent write data loss in storage layer

## Problem
When two processes write to the same key simultaneously, one write silently
disappears. This happens because the storage layer uses read-modify-write
without locking, allowing race conditions.

Reproduction test: tests/regression/test_issue_42.py

## Root Cause
[Reference: .chant/research/issue-42-root-cause.md]

The `Store.set()` method in src/storage/store.py:
1. Reads the current file contents
2. Modifies the data structure
3. Writes back to disk

Between steps 1 and 3, another process can write, causing the first write
to overwrite the second.

## Solution
Added file-based locking using fcntl to ensure atomic read-modify-write:
- Acquire exclusive lock before reading
- Perform modification
- Write and release lock

Also fixed 2 similar patterns in [list locations from sprawl phase].

## Testing
- New regression test added: tests/regression/test_issue_42.py
- Tested under high concurrency (100 concurrent writes)
- All existing tests pass
- No performance degradation measured

## Impact
- Fixes data loss affecting ~2% of concurrent writes
- Backward compatible - no API changes
- Small performance overhead (<5ms per operation)
```

### Staging PR Review

Before opening upstream PR:
1. Review the staging PR in your fork
2. Ask another maintainer to review
3. Run in staging environment for 24-48 hours if possible
4. Monitor for any unexpected behavior

### Upstream PR Creation

Only create upstream PR when:
- [ ] Technical review is complete
- [ ] Documentation is clear and complete
- [ ] Staging testing shows no issues
- [ ] Timing is appropriate (not during freeze, etc.)
- [ ] You have confidence this is the right fix

## Why This Gate Exists

This phase is NOT automated because it requires human judgment:
- **Correctness** - Is this really the right fix?
- **Completeness** - Did we miss anything?
- **Communication** - Will maintainers understand the PR?
- **Timing** - Is now the right time to submit this?
- **Politics** - Are there project considerations we should know?

An agent can implement a fix, but a human should decide when and how to
contribute it upstream.

## Acceptance Criteria

This spec is complete when a human has:
- [ ] Reviewed all items in the technical verification checklist
- [ ] Verified the staging PR in the fork
- [ ] Prepared the PR description using the template
- [ ] Decided whether to open the upstream PR now or later
- [ ] If opening PR: Created it and linked it in this spec
- [ ] If deferring PR: Documented why and when to revisit
