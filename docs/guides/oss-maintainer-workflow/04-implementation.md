# Implementation

Develop an informed fix based on thorough root cause analysis.

## Why Informed Implementation?

With research complete, implementation becomes straightforward:

- **No guessing** — You know exactly what to change
- **Confidence** — The approach has been evaluated against alternatives
- **Focus** — Edge cases are already identified
- **Testability** — The reproduction test validates your fix

## Implementation Workflow

```
Research Output      Implementation Spec       Working Fix        Review
      │                     │                      │                │
      ▼                     ▼                      ▼                ▼
┌───────────────┐     ┌───────────┐         ┌──────────┐     ┌───────────┐
│ RCA Document  │     │ Run       │         │ Code     │     │ Validated │
│ • Root cause  │────▶│ implement │────────▶│ changes  │────▶│ fix with  │
│ • Recommended │     │ prompt    │         │ + tests  │     │ passing   │
│   approach    │     └───────────┘         │ pass     │     │ tests     │
└───────────────┘                           └──────────┘     └───────────┘
```

## Creating an Implementation Spec

```bash
chant add "Fix issue #1234: Add locking to concurrent writes" --type code
```

Edit the spec to reference the research:

```yaml
---
type: code
status: ready
prompt: implement
labels:
  - fix
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-003-ghi.md     # Research spec
  - .chant/research/issue-1234-rca.md       # RCA document
target_files:
  - src/storage/store.rs
  - tests/storage/concurrent_test.rs
---

# Fix issue #1234: Add locking to concurrent writes

## Context

Research spec (2026-01-29-003-ghi) identified root cause and recommended
pessimistic locking approach. See `.chant/research/issue-1234-rca.md`
for full analysis.

## Approach

Implement pessimistic locking as recommended:
1. Acquire lock before read-modify-write cycle
2. Use existing `Lock` module
3. Handle edge cases identified in research

## Acceptance Criteria

- [ ] Lock acquired before write operation
- [ ] Reproduction test (`tests/regression/issue_1234_test.rs`) passes
- [ ] New concurrency tests added
- [ ] All existing tests pass
- [ ] Edge cases handled (timeout, nested writes, partial failures)
- [ ] Documentation updated
- [ ] Release notes section added

## Release Notes

<!-- Fill in after implementation -->
```

## The Implement Prompt

The `implement` prompt instructs the agent to follow the research:

```markdown
You are implementing a fix based on thorough root cause analysis.

Your goal is to:
1. Carefully review the research spec output (informed_by)
2. Implement the recommended approach
3. Write minimal yet clear tests that prove the fix works
4. Don't over-engineer - solve the specific problem
5. Add clear comments for non-obvious logic
6. Update relevant documentation

Instructions:
- Start by reading the research spec's recommended approach
- Verify the failing test from reproduction spec now passes
- Run full test suite to check for regressions
- Keep changes focused - don't refactor unrelated code
- Add a "Release Notes" section in the spec describing user impact

Output:
- Working fix with passing tests
- All tests passing (including reproduction test)
- Documentation updates
- Release notes section in spec
```

## Connecting Research to Implementation

The key difference from ad-hoc fixing is the explicit connection:

```yaml
informed_by:
  - .chant/specs/2026-01-29-003-ghi.md  # Research spec
```

The agent reads the research first, which provides:
- Why the recommended approach was chosen
- What edge cases to handle
- Which files to modify
- What trade-offs were considered

## Implementation Best Practices

### Follow the Research

Don't improvise. The research spec contains the analysis:

```rust
// GOOD: Implements the recommended pessimistic locking
fn write(&self, key: &str, value: &str) -> Result<()> {
    // Lock before read-modify-write (per RCA recommendation)
    let _guard = self.lock.acquire(key)?;
    let current = self.read(key)?;
    let version = current.version + 1;
    self.persist(key, value, version)
}
```

```rust
// BAD: Different approach than research recommended
fn write(&self, key: &str, value: &str) -> Result<()> {
    // Using CAS instead of locking (not what research recommended)
    self.cas_write(key, value)  // Why diverge from analysis?
}
```

If you discover the recommended approach won't work, go back to research.

### Verify Against Reproduction

The reproduction test is your success criterion:

```bash
# First, verify the test still fails
cargo test issue_1234 -- --nocapture
# Should fail

# After implementing fix
cargo test issue_1234 -- --nocapture
# Should pass
```

### Add Comprehensive Tests

Beyond the reproduction test, add tests for edge cases identified in research:

```rust
#[test]
fn concurrent_write_timeout_handling() {
    // Edge case from research: lock timeout
    let store = Store::with_timeout(Duration::from_millis(10));
    // ... test timeout behavior
}

#[test]
fn concurrent_write_nested() {
    // Edge case from research: nested writes
    let store = Store::new_temp();
    // ... test reentrant locking
}

#[test]
fn concurrent_write_partial_failure() {
    // Edge case from research: partial failures
    let store = Store::with_failing_persist();
    // ... test cleanup on failure
}
```

### Keep Changes Focused

Only modify what the research identified:

```diff
// GOOD: Minimal change
fn write(&self, key: &str, value: &str) -> Result<()> {
+   let _guard = self.lock.acquire(key)?;
    let current = self.read(key)?;
    let version = current.version + 1;
    self.persist(key, value, version)
}

// BAD: Unrelated refactoring
fn write(&self, key: &str, value: &str) -> Result<()> {
+   let _guard = self.lock.acquire(key)?;
-   let current = self.read(key)?;
+   let current = self.read_with_retry(key, 3)?;  // Unrelated change
-   let version = current.version + 1;
+   let version = self.next_version(current);     // Unrelated refactor
    self.persist(key, value, version)
}
```

### Add Clear Comments

For non-obvious logic, reference the research:

```rust
fn write(&self, key: &str, value: &str) -> Result<()> {
    // Pessimistic lock required to prevent data loss during concurrent writes.
    // See: .chant/research/issue-1234-rca.md for analysis of why optimistic
    // locking was insufficient.
    let _guard = self.lock.acquire(key)?;

    let current = self.read(key)?;
    let version = current.version + 1;
    self.persist(key, value, version)
}
```

## Release Notes Section

After implementing, add release notes to the spec:

```markdown
## Release Notes

### Fixed: Data loss during concurrent writes (#1234)

Previously, when multiple processes wrote to the same key simultaneously,
one write could be silently lost. This occurred because the storage layer
used optimistic locking that didn't properly serialize concurrent updates.

**Impact:** Users who experienced data loss during concurrent CLI invocations
should see consistent behavior after upgrading.

**Technical details:** The storage layer now uses pessimistic locking for
write operations. This may slightly increase write latency under high
concurrency but guarantees data integrity.

**Migration:** No action required. The fix is transparent to users.
```

## Spec Completion

When implementation is complete:

```yaml
---
type: code
status: completed
prompt: implement
labels:
  - fix
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-003-ghi.md
  - .chant/research/issue-1234-rca.md
target_files:
  - src/storage/store.rs
  - tests/storage/concurrent_test.rs
model: claude-sonnet-4-20250514
completed_at: 2026-01-29T18:00:00Z
---

# Fix issue #1234: Add locking to concurrent writes

## Release Notes

### Fixed: Data loss during concurrent writes (#1234)

[Release notes content here]

## Acceptance Criteria

- [x] Lock acquired before write operation
- [x] Reproduction test passes
- [x] New concurrency tests added
- [x] All existing tests pass
- [x] Edge cases handled
- [x] Documentation updated
- [x] Release notes section added
```

## When Implementation Reveals Issues

Sometimes implementation uncovers problems the research missed:

### Research Was Incomplete

```markdown
## Implementation Notes

While implementing the recommended approach, discovered that the `Lock`
module doesn't support the reentrant locking needed for nested writes.

**Decision:** Implement non-reentrant locking for now, document that
nested writes are unsupported. Created spec 2026-01-29-005-xyz to
add reentrant locking as a follow-up.
```

### Recommended Approach Doesn't Work

If the recommended approach fundamentally won't work, don't force it:

1. Stop implementation
2. Document what you discovered
3. Create a new research spec with the new findings
4. Re-evaluate approaches

```bash
# Create follow-up research spec
chant add "Re-research issue #1234: Pessimistic locking insufficient" --type task
# Reference original research and new findings
```

## Verification Before Review

Before marking complete, verify:

```bash
# 1. Reproduction test passes
cargo test issue_1234

# 2. New tests pass
cargo test concurrent_write

# 3. Full test suite passes
cargo test

# 4. No lint warnings
cargo clippy

# 5. Code formatted
cargo fmt --check
```

## See Also

- [Root Cause Analysis](03-research.md) — Previous step: understand why
- [Validation & Review](05-review.md) — Next step: independent verification
- [Complete Walkthrough](09-example.md) — See implementation in full context
