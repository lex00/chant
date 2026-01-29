---
name: implement
purpose: Implement fixes based on root cause analysis
---

# Implement Fix

You are implementing a fix based on thorough root cause analysis.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Your Goal

Implement the recommended fix from the research spec:

1. **Review the research:**
   - Read the RCA document (in `informed_by`)
   - Understand the root cause
   - Review the recommended approach
   - Note edge cases to handle

2. **Implement the fix:**
   - Follow the recommended approach
   - Keep changes minimal and focused
   - Handle identified edge cases
   - Add clear comments for non-obvious logic

3. **Write tests:**
   - Verify the reproduction test now passes
   - Add tests for edge cases from research
   - Ensure no regressions in existing tests

4. **Update documentation:**
   - Update API docs if behavior changed
   - Add comments explaining the fix rationale
   - Reference the issue number in relevant places

5. **Add release notes:**
   - Write user-facing description of the fix
   - Document any behavior changes
   - Include migration notes if needed

## Output

1. Code changes implementing the fix
2. Passing reproduction test
3. New tests for edge cases
4. Updated documentation
5. Release notes section in the spec

## Instructions

1. Read research spec and RCA document first
2. Implement the recommended approach exactly
3. Verify reproduction test passes after fix
4. Run full test suite to check for regressions
5. Add release notes section to the spec
6. Mark acceptance criteria as complete in `{{spec.path}}`
7. Commit with message: `chant({{spec.id}}): <description>`

## Release Notes Format

Add this section to the spec after implementation:

```markdown
## Release Notes

### Fixed: Brief description (#issue-number)

User-facing description of what was fixed and why it matters.

**Impact:** Who is affected and how this helps them.

**Technical details:** Brief technical explanation (optional).

**Migration:** Any steps users need to take (if applicable).
```

## Code Quality

- Don't over-engineer - solve the specific problem
- Don't refactor unrelated code
- Add comments only where logic isn't self-evident
- Follow existing code style and patterns
- Keep changes reviewable (minimal diff)

## Constraints

- Follow the research recommendation unless you find it won't work
- If research approach won't work, stop and create a new research spec
- Only modify files identified in the research
- Verify all tests pass before marking complete
- Always include release notes
