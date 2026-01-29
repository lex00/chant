# Validation and Review

Independent verification that the fix solves the original problem.

## Why Independent Review?

Implementation was informed by research, but still needs validation:

- **Fresh perspective** catches assumptions the implementer made
- **Regression check** ensures nothing else broke
- **Quality gate** before merging to main
- **Documentation** of review process for auditability

A review spec provides systematic validation and gates merge approval.

## Review Workflow

```
Implementation       Review Spec          Approval           Merge
    Output               │                   │                 │
      │                  ▼                   ▼                 ▼
      ▼            ┌───────────┐       ┌──────────┐      ┌─────────┐
┌──────────┐       │ Run       │       │ Approve  │      │ chant   │
│ Working  │──────▶│ review    │──────▶│ or       │─────▶│ merge   │
│ fix      │       │ prompt    │       │ reject   │      │ --all   │
└──────────┘       └───────────┘       └──────────┘      └─────────┘
                         │
                         ├── Check reproduction test
                         ├── Run full test suite
                         └── Review code quality
```

## Creating a Review Spec

```bash
chant add "Review fix for issue #1234" --type task
```

Edit the spec to reference all relevant context:

```yaml
---
type: task
status: ready
prompt: review
labels:
  - review
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-002-def.md     # Reproduction spec
  - .chant/specs/2026-01-29-004-jkl.md     # Implementation spec
  - tests/regression/issue_1234_test.rs     # The reproduction test
  - src/storage/store.rs                    # Changed file
target_files:
  - .chant/reviews/issue-1234-review.md
---

# Review fix for issue #1234

## Context

Implementation spec (2026-01-29-004-jkl) claims to fix concurrent write
data loss. This review validates the fix independently.

## Review Checklist

- [ ] Reproduction test passes
- [ ] All existing tests pass
- [ ] New tests adequately cover the fix
- [ ] Code changes match the research recommendation
- [ ] No obvious regressions introduced
- [ ] Documentation updated appropriately
- [ ] Release notes are clear and accurate

## Acceptance Criteria

- [ ] Review document produced with clear verdict
- [ ] All checklist items verified
- [ ] Approval or rejection with justification
```

## The Review Prompt

The `review` prompt instructs the agent to validate independently:

```markdown
You are conducting an independent review of a proposed fix.

Your goal is to:
1. Verify the fix solves the original issue (check reproduction spec)
2. Check for regressions (run full test suite)
3. Validate test coverage is adequate
4. Review code quality and maintainability
5. Ensure documentation is updated

Instructions:
- Start by reading the original issue and reproduction spec
- Verify the previously-failing test now passes
- Run full test suite and check for unexpected failures
- Read the implementation for clarity and correctness
- Check that edge cases from research spec are handled

Output:
- Approval/rejection decision with clear reasoning
- List of any issues found
- Suggestions for improvement (if any)
```

## Review Output

A review spec produces a review document:

```markdown
# Review: Issue #1234 Fix

**Date:** 2026-01-29
**Reviewed by:** chant agent
**Implementation spec:** 2026-01-29-004-jkl

## Summary

**Verdict: APPROVED**

The fix correctly addresses the root cause identified in research.
All tests pass, and the implementation follows the recommended approach.

## Checklist Results

| Check | Result | Notes |
|-------|--------|-------|
| Reproduction test | ✅ Pass | `issue_1234_concurrent_write_loses_data` now passes |
| Existing tests | ✅ Pass | 247 tests, 0 failures |
| New test coverage | ✅ Adequate | 3 new tests for edge cases |
| Matches research | ✅ Yes | Uses pessimistic locking as recommended |
| No regressions | ✅ Clear | No new test failures, no performance regression |
| Documentation | ✅ Updated | `docs/architecture/storage.md` updated |
| Release notes | ✅ Clear | Describes user impact accurately |

## Test Results

```
running 250 tests
...
test regression::issue_1234_concurrent_write_loses_data ... ok
test storage::concurrent_write_timeout_handling ... ok
test storage::concurrent_write_nested ... ok
test storage::concurrent_write_partial_failure ... ok
...
test result: ok. 250 passed; 0 failed
```

## Code Review Notes

### Strengths

1. Minimal change that fixes the issue
2. Clear comment explaining the locking rationale
3. Proper RAII pattern for lock guard

### Minor Suggestions (non-blocking)

1. Consider adding `#[must_use]` to the lock guard
2. The timeout value could be configurable

## Verdict

**APPROVED** — Ready for merge.

The fix is correct, well-tested, and follows the research recommendations.
Minor suggestions can be addressed in follow-up work if desired.
```

## Approval Workflow

After review, the approval flow depends on your configuration:

### Manual Approval

```bash
# Review spec recommends approval
chant approve 2026-01-29-004-jkl --by "reviewer-name"

# Then merge
chant merge --all
```

### Rejection

If review finds issues:

```markdown
## Verdict

**REJECTED** — Issues found

### Blocking Issues

1. **Test gap:** No test for lock timeout scenario
   - Required: Add test for `LockTimeout` error handling

2. **Missing error handling:** If lock acquisition fails, error is not
   propagated correctly
   - Required: Fix error propagation in `write()` method

### Next Steps

1. Address blocking issues
2. Re-run review spec
3. Request re-approval
```

```bash
# Resume implementation spec to fix issues
chant resume 2026-01-29-004-jkl --work

# After fixes, re-run review
chant work 2026-01-29-005-mno  # Review spec
```

## Enabling Approval Requirements

Configure automatic approval requirements for agent work:

```yaml
# .chant/config.md
---
approval:
  require_approval_for_agent_work: true
---
```

With this setting:
- Specs completed by agents automatically set `approval.required: true`
- `chant merge` blocks until approved
- Provides human checkpoint for all agent changes

## Review Types

### Standard Review

For typical bug fixes and features:

```yaml
informed_by:
  - <reproduction-spec>
  - <implementation-spec>
```

### Security Review

For security-sensitive changes, add extra scrutiny:

```yaml
labels:
  - review
  - security
informed_by:
  - <implementation-spec>
  - docs/security/guidelines.md
```

And update acceptance criteria:

```markdown
## Security-Specific Checks

- [ ] No new attack vectors introduced
- [ ] Input validation adequate
- [ ] Error messages don't leak sensitive info
- [ ] Follows principle of least privilege
```

### Performance Review

For performance-sensitive changes:

```yaml
labels:
  - review
  - performance
```

```markdown
## Performance-Specific Checks

- [ ] Benchmark results compared to baseline
- [ ] No O(n²) or worse algorithms introduced
- [ ] Memory allocation patterns reasonable
- [ ] No blocking operations in hot paths
```

## Batch Review

When multiple related fixes need review:

```bash
# Create review specs with shared label
chant add "Review fix for issue #1234" --type task --label review-batch-01
chant add "Review fix for issue #1235" --type task --label review-batch-01
chant add "Review fix for issue #1236" --type task --label review-batch-01

# Execute reviews in parallel
chant work --parallel --label review-batch-01

# Approve all that passed review
chant approve 2026-01-29-004-jkl --by "reviewer"
chant approve 2026-01-29-004-xyz --by "reviewer"
```

## Review vs Self-Review

### Why Not Self-Review?

The implementation agent shouldn't review its own work:

- Same blind spots that caused bugs
- Confirmation bias toward own code
- No fresh perspective

### Separate Agent Execution

Review specs run in separate agent sessions:

```bash
# Implementation completes
chant work <impl-spec>

# Review runs independently (fresh agent context)
chant work <review-spec>
```

## Spec Completion

When review is complete:

```yaml
---
type: task
status: completed
prompt: review
labels:
  - review
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-002-def.md
  - .chant/specs/2026-01-29-004-jkl.md
target_files:
  - .chant/reviews/issue-1234-review.md
model: claude-sonnet-4-20250514
completed_at: 2026-01-29T20:00:00Z
---
```

## See Also

- [Implementation](04-implementation.md) — Previous step: develop the fix
- [Documentation](06-documentation.md) — Next step: update user-facing docs
- [Advanced Patterns](08-advanced.md) — Security and breaking change reviews
