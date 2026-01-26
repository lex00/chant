---
type: group
status: completed
completed_at: 2026-01-25T20:41:26Z
model: auto-completed
---
# Swift 6 Upgrade

Upgrade SwiftyJSON to compile and run correctly with Swift 6, including full Sendable support for the JSON type.

## Background

SwiftyJSON (23k stars) is still on Swift 5.3. GitHub issue #1163 requests Swift 6 / Sendable support. The library compiles with Swift 6 tools but fails when JSON is used across actor boundaries.

## Acceptance Criteria

- [ ] Package.swift updated to swift-tools-version:6.0
- [ ] swiftLanguageVersions lock removed from Package.swift
- [ ] JSON struct conforms to Sendable
- [ ] SwiftyJSONError enum conforms to Sendable
- [ ] Type enum conforms to Sendable
- [ ] All 147 existing tests pass
- [ ] New concurrency test verifies JSON works across actor boundaries
- [ ] No compiler warnings