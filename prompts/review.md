---
name: review
purpose: Independent review of proposed fixes
---

# Review Fix

You are conducting an independent review of a proposed fix.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Your Goal

Validate the fix independently and thoroughly:

1. **Understand the original issue:**
   - Read the reproduction spec (in `informed_by`)
   - Understand what the failing test demonstrated
   - Know what "fixed" should look like

2. **Verify the fix works:**
   - Run the reproduction test - it should pass now
   - Run the full test suite - no regressions
   - Check that new tests cover edge cases

3. **Review code quality:**
   - Does the implementation match the research recommendation?
   - Is the code clear and maintainable?
   - Are comments adequate for non-obvious logic?
   - Is the change minimal and focused?

4. **Check documentation:**
   - Are API docs updated if needed?
   - Are release notes clear and accurate?
   - Is migration guidance provided for breaking changes?

5. **Make a decision:**
   - APPROVE: Fix is correct and ready to merge
   - REJECT: Issues found that must be addressed

## Output

Create a review document at the target file location with:

1. **Verdict:** APPROVED or REJECTED
2. **Checklist Results:** Table of verification results
3. **Test Results:** Summary of test runs
4. **Code Review Notes:** Observations about the implementation
5. **Issues Found:** List of problems (if rejecting)
6. **Suggestions:** Optional improvements (non-blocking)

## Review Checklist

| Check | Result | Notes |
|-------|--------|-------|
| Reproduction test passes | | |
| All existing tests pass | | |
| New tests adequate | | |
| Matches research recommendation | | |
| No obvious regressions | | |
| Documentation updated | | |
| Release notes clear | | |

## Instructions

1. Read the original reproduction spec first
2. Run the reproduction test to verify it passes
3. Run the full test suite
4. Review the code changes for correctness
5. Check documentation and release notes
6. Write review document with clear verdict
7. Mark acceptance criteria as complete in `{{spec.path}}`
8. Commit with message: `chant({{spec.id}}): <description>`

## Approval Criteria

Approve if ALL of these are true:
- Reproduction test passes
- No test regressions
- Code is correct and follows recommendation
- Edge cases are handled
- Documentation is adequate

## Rejection Criteria

Reject if ANY of these are true:
- Reproduction test still fails
- New test failures introduced
- Code doesn't match research recommendation without justification
- Critical edge cases unhandled
- Missing or incorrect documentation

## Constraints

- Be objective - don't approve to be nice
- Be specific about issues found
- Distinguish blocking issues from suggestions
- Don't reject for style preferences alone
- Verify claims by running tests yourself
