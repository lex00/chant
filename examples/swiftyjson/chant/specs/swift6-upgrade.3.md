---
type: code
status: completed
depends_on:
- swift6-upgrade.2
target_files:
- Source/SwiftyJSON/SwiftyJSON.swift
commits:
- f8bdd7c
completed_at: 2026-01-25T20:24:57Z
---
# Add Sendable Conformance to SwiftyJSONError Enum

Make the SwiftyJSONError enum conform to Sendable so errors can be safely thrown and caught across actor boundaries in Swift 6 concurrency.

### Acceptance Criteria

- [ ] SwiftyJSONError enum at line 27 has explicit `: Sendable` conformance added
- [ ] No compiler warnings about Sendable conformance for the error type
- [ ] Error's CustomNSError conformance (line 36) remains functional
- [ ] Code compiles successfully with `swift build`
- [ ] All existing tests still pass with `swift test`
- [ ] Error can be thrown across actor boundaries without warnings

### Edge Cases

- Verify that the Int raw value and CustomNSError conformance don't conflict with Sendable
- Check that the errorUserInfo computed property (returns `[String: Any]`) doesn't cause Sendable issues - it shouldn't since it's computed, not stored
- Ensure that NSLocalizedDescriptionKey and string literals in the switch cases are Sendable-compatible

### Example Test Cases

Verify:
- SwiftyJSONError can be thrown from async functions
- Errors can be caught and inspected across actor boundaries
- Error userInfo dictionary is accessible in concurrent contexts
- No warnings when using `throw SwiftyJSONError.invalidJSON` in async contexts

## Agent Output

2026-01-25T20:24:57Z

```
I'll help you implement the spec to add Sendable conformance to the SwiftyJSONError enum. Let me start by reading the relevant code and planning the approach.
Let me find the SwiftyJSONError enum file:
Now let me read the main SwiftyJSON file where the error should be defined:
Perfect! I found the SwiftyJSONError enum at line 27. Now I can see the current definition and understand what needs to be done. Let me create a plan and then implement the change.
Now let me make the change to add Sendable conformance to SwiftyJSONError:
Great! I've successfully added Sendable conformance to the SwiftyJSONError enum. Now let me update the todo list and verify the build:
Excellent! The build completed successfully. Now let me run the tests:
Let me check the full test results to make sure all tests passed:
Let me get the actual test result summary:
Perfect! All 146 tests passed. Now let me update the todo list and commit the changes:
Perfect! I have successfully completed the spec to add Sendable conformance to the SwiftyJSONError enum. Here's a summary of what was accomplished:

## Implementation Summary

**Change Made:**
- Modified `SwiftyJSONError` enum at line 27 of `/Users/alex/Documents/checkouts/SwiftyJSON/Source/SwiftyJSON/SwiftyJSON.swift`
- Added `: Sendable` conformance to make it: `public enum SwiftyJSONError: Int, Swift.Error, Sendable`

**Verification:**
✅ **Code compiles successfully** - `swift build` completed without errors or warnings
✅ **All 146 tests pass** - Executed with 0 failures
✅ **Existing conformances maintained** - The Int raw value and CustomNSError conformance (line 36) remain fully functional
✅ **Sendable compatible** - The enum contains only Sendable-compatible components:
   - Int raw value (Sendable)
   - String literals in switch cases (Sendable)
   - Computed `errorUserInfo` property returns `[String: Any]` which is safe since it's computed, not stored

**Commit Created:**
```
chant(swift6-upgrade.3): Add Sendable conformance to SwiftyJSONError enum
```

The SwiftyJSONError can now be safely thrown and caught across actor boundaries in Swift 6 concurrency without any warnings.```
