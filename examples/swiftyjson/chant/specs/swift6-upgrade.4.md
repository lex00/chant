---
type: code
status: completed
depends_on:
- swift6-upgrade.3
target_files:
- Source/SwiftyJSON/SwiftyJSON.swift
commits:
- '7636e36'
completed_at: 2026-01-25T20:43:46Z
---
# Add Sendable Conformance to JSON Struct

Make the JSON struct conform to Sendable by analyzing its stored properties and ensuring thread-safety. This is the most complex member as JSON stores various reference types (NSNumber, arrays, dictionaries) that need careful consideration.

### Acceptance Criteria

- [ ] JSON struct at line 82 has explicit `: Sendable` conformance added
- [ ] All stored properties are verified to be Sendable-compatible or made so
- [ ] Verify rawArray (line 198) storing `[Any]` is safe - `Any` values come from Foundation JSON types which are all Sendable
- [ ] Verify rawDictionary (line 199) storing `[String: Any]` is safe
- [ ] Verify rawNumber (line 201) NSNumber is Sendable-compatible
- [ ] Verify rawNull (line 202) NSNull is Sendable-compatible
- [ ] Verify rawString (line 200) String is Sendable
- [ ] Verify rawBool (line 203) Bool is Sendable
- [ ] No compiler warnings about Sendable conformance
- [ ] Code compiles successfully with `swift build`
- [ ] All 147 existing tests pass with `swift test`

### Edge Cases

- NSNumber and NSNull are Objective-C classes - verify they're marked as Sendable in Swift 6 Foundation (they should be, as they're immutable value types)
- The `object` property (line 212) returns Any - verify this doesn't cause issues when JSON is used across actors
- The Index enum (line 274) with DictionaryIndex may need Sendable conformance
- JSONKey enum (line 342) may need Sendable conformance
- Verify that the private stored properties (rawArray, rawDictionary, etc.) containing Foundation types are truly immutable when JSON is used across actors
- Check that mutating methods (merge, subscript setters) don't create race conditions - they shouldn't since each JSON instance should not be shared mutably across actors

### Example Test Cases

Verify:
- JSON instances can be passed to actor-isolated functions
- JSON can be captured in @Sendable closures
- JSON can be stored in actor properties
- JSON array and dictionary access works across actor boundaries
- Subscript operations on JSON work in concurrent contexts
- Test case: Create JSON in one actor, pass to another actor, read values

