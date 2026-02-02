---
name: implement
purpose: Implement fix based on research findings
---

# Implement Fix

You are implementing a fix for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Implementation Process

### 1. Understand the Research

**Tracked files** (research to implement):
- These files contain the investigation and root cause analysis
- Read them completely to understand what needs to be fixed
- Understand the mechanism of the bug
- Review recommended fix approaches

### 2. Plan the Fix

Before coding:

1. **Understand the root cause**
   - What code is causing the issue?
   - Why does it fail?
   - What conditions trigger it?

2. **Review recommended approaches**
   - What fix was recommended in research?
   - Are there alternative approaches?
   - What are the trade-offs?

3. **Identify scope**
   - Which files need to change?
   - What tests need to be added or updated?
   - Are there related areas that need attention?

### 3. Implement the Fix

Follow this sequence:

1. **Read the existing code** - Understand current implementation
   - Read files that need to change
   - Understand surrounding context
   - Note existing patterns and conventions

2. **Make minimal changes** - Fix only what needs fixing
   - Follow the recommended approach from research
   - Maintain existing code style and patterns
   - Don't refactor unrelated code

3. **Add or update tests** - Ensure fix works and won't regress
   - Add test case for the bug (should fail before fix, pass after)
   - Update existing tests if behavior changed
   - Verify edge cases are covered

4. **Verify the fix**
   - Run tests with `just test`
   - Verify the reproduction case (if available) now works
   - Check for any side effects

### 4. Apply Implementation Principles

- **Focused** — Fix only the identified issue, don't refactor unrelated code
- **Minimal** — Make the smallest change that fixes the issue
- **Tested** — Include tests that verify the fix
- **Consistent** — Follow existing code patterns and style
- **Safe** — Consider edge cases and potential side effects

### 5. Verification Steps

1. **Run `cargo fmt`** - Format the code
2. **Run `cargo clippy`** - Fix any lint errors and warnings
3. **Run `just test`** - All tests must pass
4. **Verify fix** - Confirm the original issue is resolved
5. **Check side effects** - Ensure no new issues introduced

## Common Fix Patterns

### Fixing Logic Errors

```rust
// Before (buggy)
if condition {
    // Wrong logic
}

// After (fixed)
if corrected_condition {
    // Correct logic
}
```

### Fixing State Management

```rust
// Before (buggy)
// Missing state update

// After (fixed)
self.state.update(new_value);
```

### Fixing Error Handling

```rust
// Before (buggy)
result.unwrap() // Can panic

// After (fixed)
result.map_err(|e| /* proper error handling */)?
```

### Adding Missing Validation

```rust
// Before (buggy)
// No validation

// After (fixed)
if !input.is_valid() {
    return Err(Error::InvalidInput);
}
```

## Test Patterns

Always include tests for your fix:

```rust
#[test]
fn test_bug_is_fixed() {
    // Arrange: Set up the conditions that triggered the bug
    let setup = create_buggy_scenario();

    // Act: Execute the code that previously failed
    let result = execute_fixed_code(setup);

    // Assert: Verify the correct behavior
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected_value);
}
```

## Output Format

Document your implementation:

```markdown
# Implementation: [Bug Name]

## Changes Made

### File: `path/to/file.rs`

**Lines changed**: 123-145

**What changed**: Description of the modification

**Why**: Why this change fixes the issue

### File: `path/to/test.rs`

**Lines changed**: 67-89

**What changed**: Description of test added

**Why**: What this test verifies

## Verification

- [x] `cargo fmt` passed
- [x] `cargo clippy` passed with no warnings
- [x] `just test` passed all tests
- [x] Reproduction case now works correctly
- [x] No side effects observed

## Testing

Tests added or updated:
- `test_bug_is_fixed`: Verifies the bug is resolved
- `test_edge_case`: Ensures edge case is handled

## Related Issues

Any follow-up work needed:
- Related issue 1: [description]
```

### 6. Verification Checklist

Before marking complete:

1. **Fix verification**: Does the fix resolve the root cause?
2. **Test verification**: Do tests pass and cover the bug?
3. **Code quality**: Does code pass fmt and clippy?
4. **Style verification**: Does code follow project conventions?
5. **Acceptance criteria**: Are all criteria met?

## Constraints

- Follow the recommended approach from research
- Make minimal changes focused on the fix
- Don't refactor unrelated code
- Always add tests for the bug
- Run cargo fmt, clippy, and tests
- Ensure no side effects introduced

## Instructions

1. **Read** the research findings and root cause analysis
2. **Plan** the fix approach based on recommendations
3. **Read** existing code that needs modification
4. **Implement** the minimal fix
5. **Add/update** tests to verify the fix
6. **Run** `cargo fmt` to format code
7. **Run** `cargo clippy` to fix lint issues
8. **Run** `just test` and fix any failures
9. **Verify** the fix works and no side effects exist
10. **Check off** each acceptance criterion in `{{spec.path}}` by changing `- [ ]` to `- [x]`
11. **Commit** with message: `chant({{spec.id}}): <description>`
12. **Verify git status is clean** - ensure no uncommitted changes remain
