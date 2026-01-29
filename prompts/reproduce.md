---
name: reproduce
purpose: Create minimal reproduction case for reported issues
---

# Create Reproduction Case

You are creating a minimal reproduction case for a reported issue.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Your Goal

Create the smallest possible test case that demonstrates the bug:

1. **Understand the issue:**
   - Read the triage assessment (in `informed_by`)
   - Understand the reported symptoms
   - Note environment details

2. **Create a failing test:**
   - Use the project's test framework
   - Make the test self-contained (no external dependencies)
   - Make it minimal (smallest code that shows the bug)
   - Make it reliable (fails consistently, not flaky)

3. **Document the reproduction:**
   - Add issue URL in test comments
   - Include environment details
   - Describe expected vs actual behavior

4. **Verify the test fails:**
   - Run the test and confirm it fails
   - The failure should match the reported symptoms
   - If it doesn't fail, document why reproduction failed

## Test File Format

```rust
//! Regression test for issue #XXXX: Brief description
//!
//! Issue: https://github.com/project/issues/XXXX
//! Reporter: @username
//! Environment: OS, version, etc.

#[test]
fn issue_xxxx_brief_description() {
    // Setup
    // ...

    // Action that triggers the bug
    // ...

    // Assertion that fails due to the bug
    assert!(/* condition */, "Descriptive message");
}
```

## Output

1. A failing test file at the `target_files` location
2. Test should fail with a clear error message
3. Comments should document the reproduction context

## Instructions

1. Read referenced triage spec and issue details
2. Create minimal reproduction test
3. Run the test to verify it fails as expected
4. If reproduction fails, document what you tried
5. Mark acceptance criteria as complete in `{{spec.path}}`
6. Commit with message: `chant({{spec.id}}): <description>`

## Handling Failed Reproduction

If you cannot reproduce the bug:

1. Document all approaches you tried
2. List possible reasons (environment, configuration, etc.)
3. Recommend next steps (request more info, different approach)
4. Mark the spec with your findings

## Constraints

- Only create the reproduction test, don't fix the bug
- Test must be in project's standard test location
- Test name should include issue number
- Keep the test minimal and focused
