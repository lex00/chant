# Documentation Specs

Documentation as first-class specs using `type: documentation`.

## Why Documentation Specs?

Documentation is often an afterthought. Problems with ad-hoc docs:

- **Drift:** Docs become outdated as code changes
- **Incomplete:** New features ship without docs
- **Inconsistent:** Different features documented differently
- **Untraceable:** No audit trail of doc changes

Chant treats documentation as trackable, verifiable specs with drift detection.

## Documentation Workflow

```
Implementation      Drift Detection       Doc Spec           Updated Docs
    Complete              │                  │                    │
       │                  ▼                  ▼                    ▼
       ▼            ┌───────────┐      ┌───────────┐        ┌──────────┐
┌──────────────┐    │ chant     │      │ Run       │        │ Docs in  │
│ Code changes │───▶│ drift     │─────▶│ document  │───────▶│ sync     │
│ merged       │    │           │      │ prompt    │        │ with     │
└──────────────┘    └───────────┘      └───────────┘        │ code     │
                          │                                  └──────────┘
                          └── Detects outdated docs
```

## When to Use `type: documentation`

### Use Documentation Type For:

- **Drift fixes:** Update docs that are out of sync with code
- **User guides:** Write standalone tutorials and how-tos
- **Migration guides:** Document breaking changes
- **Troubleshooting:** Add FAQ entries, debugging guides
- **API docs:** Document new/changed public APIs
- **Release notes:** Changelog entries as documentation specs

### Use Code Type For:

- **Inline docs:** Docstrings, code comments (ship with code)
- **Small fixes:** Minor doc corrections alongside bug fixes
- **README updates:** When tightly coupled to code changes

**Rule of thumb:** If the docs can be verified independently from code, use `type: documentation`. If docs must ship atomically with code, bundle them in the `type: code` spec.

## Drift Detection

Chant detects when documentation references code that has changed:

```bash
$ chant drift

Documentation drift detected:

  docs/api/storage.md
    References: src/storage/store.rs (modified 3 days ago)
    Last updated: 2026-01-15

  docs/guides/concurrency.md
    References: src/storage/concurrent.rs (modified 3 days ago)
    Last updated: 2026-01-10

Recommendation: Create documentation specs to update drifted docs.
```

### How Drift Is Detected

Chant compares:
- `origin:` files in completed specs (when they last ran)
- `informed_by:` references in documentation
- Git modification dates

When referenced files change after docs were last updated, drift is flagged.

## Creating a Documentation Spec

### From Drift Detection

```bash
# Detect drift
chant drift

# Create spec to fix drift
chant add "Update storage API documentation" --type documentation
```

Edit the spec:

```yaml
---
type: documentation
status: ready
prompt: document
labels:
  - docs-drift
  - storage
informed_by:
  - .chant/specs/2026-01-29-004-jkl.md  # Implementation that changed API
  - src/storage/store.rs                 # New API to document
target_files:
  - docs/api/storage.md
  - docs/guides/concurrency.md
---

# Update storage API documentation

## Context

The storage API changed in spec 2026-01-29-004-jkl to add locking for
concurrent writes. The documentation still describes the old behavior.

## Drift Details

- `docs/api/storage.md` references old non-locking write behavior
- `docs/guides/concurrency.md` missing new locking information

## Acceptance Criteria

- [ ] `docs/api/storage.md` updated with new `write()` behavior
- [ ] Locking semantics documented
- [ ] `docs/guides/concurrency.md` updated with locking example
- [ ] Code examples tested and correct
```

### From Implementation

After implementing a fix, create a doc spec if needed:

```yaml
---
type: documentation
status: ready
prompt: document
labels:
  - docs
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-004-jkl.md  # Implementation spec
target_files:
  - docs/api/storage.md
---

# Document concurrent write locking API

## Context

Implementation spec 2026-01-29-004-jkl added locking to the write API.
Users need to understand the new behavior.

## Acceptance Criteria

- [ ] Document that `write()` now blocks under concurrency
- [ ] Explain timeout behavior
- [ ] Add troubleshooting section for common issues
- [ ] Update API reference with new behavior
```

## The Document Prompt

The `document` prompt instructs the agent to write user-facing docs:

```markdown
You are writing user-facing documentation for a fix or feature.

Your goal is to:
1. Update guides, API docs, and examples
2. Write migration guides for breaking changes
3. Add troubleshooting entries if relevant
4. Update FAQ if needed
5. Ensure examples are clear and correct

Instructions:
- Review the implementation spec to understand what changed
- Write for users, not implementers (focus on what, not how)
- Include code examples where helpful
- Highlight breaking changes prominently
- Update table of contents and cross-references

Output:
- Updated documentation files
- Migration guide (if breaking changes)
- Code examples demonstrating the change
```

## Types of Documentation Specs

### 1. Drift Fixes

Update docs that fell out of sync:

```yaml
---
type: documentation
labels:
  - docs-drift
informed_by:
  - src/storage/store.rs  # Changed code
---

# Fix drift in storage documentation

Drift detected: `docs/api/storage.md` references behavior that changed
in commit abc123.
```

### 2. API Documentation

Document new or changed public APIs:

```yaml
---
type: documentation
labels:
  - api-docs
informed_by:
  - .chant/specs/2026-01-29-004-jkl.md  # Feature spec
target_files:
  - docs/api/new-feature.md
---

# Document new feature API

Add comprehensive API documentation for the new concurrent write feature.
```

### 3. User Guides

Write tutorials and how-tos:

```yaml
---
type: documentation
labels:
  - guide
target_files:
  - docs/guides/getting-started.md
---

# Write getting started guide

Create beginner-friendly guide for new users.
```

### 4. Migration Guides

Document breaking changes:

```yaml
---
type: documentation
labels:
  - migration
  - breaking-change
informed_by:
  - .chant/specs/2026-01-29-010-xyz.md  # Breaking change spec
target_files:
  - docs/migration/v2-to-v3.md
---

# Write v2 to v3 migration guide

Document breaking changes and migration path for major version upgrade.

## Breaking Changes to Document

- [ ] `write()` now blocks (previously async)
- [ ] New `LockTimeout` error type
- [ ] Configuration file format change
```

### 5. Troubleshooting

Add FAQ and debugging guidance:

```yaml
---
type: documentation
labels:
  - troubleshooting
informed_by:
  - https://github.com/yourproject/issues?q=label:question
target_files:
  - docs/troubleshooting/common-issues.md
---

# Add troubleshooting entries

Based on common support questions, add troubleshooting documentation.
```

### 6. Release Notes

Changelog entries as specs:

```yaml
---
type: documentation
labels:
  - release-notes
  - v0.7.0
informed_by:
  - .chant/specs/2026-01-29-004-jkl.md
  - .chant/specs/2026-01-29-005-mno.md
target_files:
  - CHANGELOG.md
---

# Write release notes for v0.7.0

Aggregate all changes into user-friendly release notes.
```

## Documentation Spec Workflow

Complete workflow for handling documentation:

```bash
# 1. After implementation completes, check for drift
chant drift

# 2. Create documentation spec
chant add "Fix drift in storage docs" --type documentation

# 3. Edit spec to reference implementation
# Add to spec:
#   informed_by:
#     - .chant/specs/2026-01-29-004-jkl.md
#     - src/storage/store.rs

# 4. Work the documentation spec
chant work <doc-spec-id> --prompt document

# 5. Review documentation changes
chant add "Review storage documentation updates" --type task
chant work <review-spec-id>

# 6. Approve and merge
chant approve <doc-spec-id> --by "reviewer-name"
chant merge --all
```

## Documentation Output Example

A documentation spec might produce:

```markdown
<!-- docs/api/storage.md -->

## write(key, value)

Writes a value to the store with the given key.

### Behavior

The `write()` method uses pessimistic locking to ensure data integrity
during concurrent operations. When multiple threads or processes attempt
to write to the same key:

1. Each caller acquires an exclusive lock
2. Callers queue behind current lock holder
3. Writes proceed one at a time

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `key` | `&str` | The key to write to |
| `value` | `&str` | The value to store |

### Returns

`Result<(), WriteError>`

### Errors

| Error | Cause |
|-------|-------|
| `WriteError::LockTimeout` | Lock not acquired within timeout period |
| `WriteError::Io(e)` | Underlying I/O error |

### Example

```rust
use yourproject::Store;

let store = Store::open("mydb")?;

// Single write
store.write("key", "value")?;

// Concurrent writes are serialized automatically
let handle1 = {
    let store = store.clone();
    std::thread::spawn(move || store.write("key", "value1"))
};
let handle2 = {
    let store = store.clone();
    std::thread::spawn(move || store.write("key", "value2"))
};

// Both writes succeed, one value wins (last writer)
handle1.join().unwrap()?;
handle2.join().unwrap()?;
```

### Timeout Configuration

Default lock timeout is 30 seconds. Configure with:

```rust
let store = Store::builder()
    .lock_timeout(Duration::from_secs(60))
    .open("mydb")?;
```

### See Also

- [Concurrency Guide](../guides/concurrency.md)
- [Troubleshooting: Lock Timeout](../troubleshooting/common-issues.md#lock-timeout)
```

## Bundled vs Separate Documentation

### Bundled (in Code Spec)

For tightly coupled docs:

```yaml
---
type: code
target_files:
  - src/storage/store.rs
  - docs/api/storage.md  # Docs bundled with code
---
```

**Pros:**
- Atomic: code and docs ship together
- Single review

**Cons:**
- Larger spec scope
- Doc changes can't proceed independently

### Separate (Documentation Spec)

For independent docs:

```yaml
---
type: code
target_files:
  - src/storage/store.rs
---

# Then separately:

---
type: documentation
depends_on:
  - <code-spec-id>
target_files:
  - docs/api/storage.md
---
```

**Pros:**
- Documentation can be refined independently
- Different reviewers for code vs docs
- Docs can catch up after multiple code specs

**Cons:**
- Requires coordination
- Risk of forgetting to update docs

## Spec Completion

When documentation is complete:

```yaml
---
type: documentation
status: completed
prompt: document
labels:
  - docs-drift
  - storage
informed_by:
  - .chant/specs/2026-01-29-004-jkl.md
  - src/storage/store.rs
target_files:
  - docs/api/storage.md
  - docs/guides/concurrency.md
model: claude-sonnet-4-20250514
completed_at: 2026-01-30T10:00:00Z
---
```

## See Also

- [Validation & Review](05-review.md) — Previous step: review the implementation
- [Release Coordination](07-release.md) — Next step: aggregate into release notes
- [Research Workflows Guide](../research.md) — Using `origin:` and `informed_by:` for drift
