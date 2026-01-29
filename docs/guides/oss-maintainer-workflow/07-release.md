# Release Coordination

Aggregate multiple fixes into user-friendly release notes.

## Why Release Specs?

Individual specs have release notes sections, but releases need:

- **Aggregation:** Combine notes from multiple specs
- **User perspective:** Translate technical changes for users
- **Organization:** Group by category (fixes, features, breaking changes)
- **Completeness:** Ensure nothing is missed

A release spec coordinates this effort.

## Release Workflow

```
Completed Specs       Release Spec          Release Notes         Ship
       │                   │                      │                 │
       ▼                   ▼                      ▼                 ▼
┌───────────────┐    ┌───────────┐         ┌───────────┐      ┌─────────┐
│ Multiple      │    │ Aggregate │         │ CHANGELOG │      │ Tag     │
│ fixes/        │───▶│ release   │────────▶│ User-     │─────▶│ Release │
│ features      │    │ notes     │         │ friendly  │      │ Publish │
└───────────────┘    └───────────┘         └───────────┘      └─────────┘
```

## Creating a Release Spec

```bash
chant add "Prepare release notes for v0.7.0" --type task
```

Edit the spec to reference all completed work:

```yaml
---
type: task
status: ready
labels:
  - release
  - v0.7.0
informed_by:
  - .chant/specs/2026-01-29-004-jkl.md  # Fix: concurrent writes
  - .chant/specs/2026-01-29-005-mno.md  # Feature: lock timeout config
  - .chant/specs/2026-01-29-006-pqr.md  # Docs: storage guide
target_files:
  - CHANGELOG.md
---

# Prepare release notes for v0.7.0

## Specs to Include

| Spec | Type | Summary |
|------|------|---------|
| 004-jkl | fix | Concurrent write data loss |
| 005-mno | feature | Configurable lock timeout |
| 006-pqr | docs | Storage documentation update |

## Acceptance Criteria

- [ ] All completed specs since v0.6.0 reviewed
- [ ] Changes categorized (Breaking, Features, Fixes, Docs)
- [ ] User-facing descriptions written (not technical jargon)
- [ ] Migration notes for any breaking changes
- [ ] Upgrade instructions if needed
- [ ] CHANGELOG.md updated
```

## Gathering Completed Specs

Find all specs completed since last release:

```bash
# List completed specs since date
chant list --status completed --since 2026-01-15

# Or by label
chant list --status completed --label v0.7.0

# Export for review
chant export --status completed --since 2026-01-15 --format markdown
```

## Release Notes Structure

A well-organized CHANGELOG entry:

```markdown
## [0.7.0] - 2026-01-30

### Breaking Changes

- **Storage API:** `write()` now blocks during concurrent access.
  Previously, concurrent writes could cause data loss. The new behavior
  guarantees data integrity but may increase latency under high concurrency.

  **Migration:** No code changes required. If your application relies on
  non-blocking writes, consider using `write_async()` instead.

### Features

- **Configurable lock timeout:** Set custom timeout for write operations:
  ```rust
  Store::builder()
      .lock_timeout(Duration::from_secs(60))
      .open("mydb")?;
  ```
  Default timeout remains 30 seconds. (#1235)

### Bug Fixes

- **Fixed:** Data loss during concurrent writes. When multiple processes
  wrote to the same key simultaneously, one write could be silently lost.
  The storage layer now uses pessimistic locking to serialize writes.
  (#1234)

### Documentation

- Updated storage API documentation with new locking behavior
- Added concurrency guide with examples
- Added troubleshooting section for lock timeout errors

### Internal

- Refactored lock module for better testability
- Added comprehensive concurrency test suite
```

## User-Facing vs Technical

Transform technical changes into user-facing descriptions:

### Technical (from spec)

```markdown
Implemented pessimistic locking in store.rs write() method using
the existing Lock module. Added RAII guard pattern for lock release.
```

### User-Facing (for release notes)

```markdown
**Fixed:** Data loss during concurrent writes. When multiple processes
wrote to the same key simultaneously, one write could be silently lost.
Your data is now safe under concurrent access.
```

## Categorizing Changes

### Breaking Changes

Changes that require user action:

- API signature changes
- Removed features
- Changed default behavior
- Configuration format changes

**Always include migration instructions.**

### Features

New capabilities users can adopt:

- New APIs
- New configuration options
- New CLI commands
- Performance improvements (if user-visible)

### Bug Fixes

Corrections to existing behavior:

- Crashes fixed
- Data integrity issues resolved
- Edge cases handled
- Error messages improved

### Documentation

Documentation-only changes:

- New guides
- Updated API docs
- Fixed examples
- Improved troubleshooting

### Internal

Changes invisible to users (optional, for transparency):

- Refactoring
- Test improvements
- Build changes
- Dependency updates

## Handling Breaking Changes

For breaking changes, be thorough:

```markdown
### Breaking Changes

#### `write()` method now blocks during concurrent access

**What changed:** The `write()` method in the storage API now acquires
an exclusive lock before writing. This means concurrent callers will
block until the current write completes.

**Why:** This prevents data loss that could occur when multiple processes
wrote to the same key simultaneously. See #1234 for the original issue.

**Who is affected:** Applications that rely on `write()` being non-blocking,
or that use `write()` in performance-critical paths with high concurrency.

**Migration:**

1. **No action required** if write latency under concurrency is acceptable.

2. **For non-blocking writes**, use the new `write_async()` method:
   ```rust
   // Before
   store.write("key", "value")?;

   // After (if non-blocking needed)
   store.write_async("key", "value").await?;
   ```

3. **For timeout configuration**, use the builder:
   ```rust
   Store::builder()
       .lock_timeout(Duration::from_secs(60))
       .open("mydb")?;
   ```

**Rollback:** If you experience issues, pin to version 0.6.x until your
application can be updated.
```

## Release Notes from Multiple Issues

When a release includes many fixes:

```yaml
informed_by:
  - .chant/specs/2026-01-29-004-jkl.md  # Issue #1234
  - .chant/specs/2026-01-29-007-stu.md  # Issue #1235
  - .chant/specs/2026-01-29-008-vwx.md  # Issue #1236
  - .chant/specs/2026-01-29-009-yza.md  # Issue #1237
```

The release spec aggregates all release notes sections:

```markdown
### Bug Fixes

- **Fixed:** Data loss during concurrent writes (#1234)
- **Fixed:** Unicode filenames not handled correctly (#1235)
- **Fixed:** Crash on empty configuration file (#1236)
- **Fixed:** Memory leak in long-running processes (#1237)
```

## SemVer Considerations

Determine version number based on changes:

| Change Type | Version Bump | Example |
|-------------|--------------|---------|
| Breaking change | Major | 0.6.0 → 1.0.0 |
| New feature | Minor | 0.6.0 → 0.7.0 |
| Bug fix only | Patch | 0.6.0 → 0.6.1 |

For pre-1.0 projects:
- Breaking changes bump minor (0.6.0 → 0.7.0)
- Features bump minor (0.6.0 → 0.7.0)
- Fixes bump patch (0.6.0 → 0.6.1)

## Release Checklist

Include a pre-release checklist in acceptance criteria:

```markdown
## Pre-Release Checklist

- [ ] All specs for this release are merged
- [ ] CHANGELOG.md updated
- [ ] Version number bumped in Cargo.toml
- [ ] All tests passing on main branch
- [ ] Documentation published
- [ ] GitHub release draft created
- [ ] Migration guide reviewed (if breaking changes)
```

## Spec Completion

When release notes are complete:

```yaml
---
type: task
status: completed
labels:
  - release
  - v0.7.0
informed_by:
  - .chant/specs/2026-01-29-004-jkl.md
  - .chant/specs/2026-01-29-005-mno.md
  - .chant/specs/2026-01-29-006-pqr.md
target_files:
  - CHANGELOG.md
model: claude-sonnet-4-20250514
completed_at: 2026-01-30T12:00:00Z
---
```

## Post-Release

After the release is published:

```bash
# Archive completed specs
chant archive --label v0.7.0

# Or archive individually
chant archive 2026-01-29-004-jkl
chant archive 2026-01-29-005-mno
```

## See Also

- [Documentation](06-documentation.md) — Previous step: update docs
- [Advanced Patterns](08-advanced.md) — Coordinating multiple releases
- [Complete Walkthrough](09-example.md) — See release in full context
