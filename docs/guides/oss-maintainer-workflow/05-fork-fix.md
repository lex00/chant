# Phase 5: Fork Fix - Implement fix for Issue #1234

Implement the fix in your fork and create a fork-internal PR for review before submitting upstream.

## Why Fork-Internal Staging PR?

For OSS maintainers, the staging PR pattern provides:

- **Review isolation** — Review the fix in your fork before exposing it upstream
- **Iteration space** — Make changes without upstream visibility
- **Quality gate** — Ensure CI passes before creating upstream PR
- **Clean history** — Squash and polish commits before going upstream

The staging PR is internal to your fork (e.g., `yourfork:fix/issue-1234` → `yourfork:main`), not to upstream.

## Fork Fix Workflow

```
Research        Implementation       Working Fix      Staging PR       Human Gate
 Output              Spec                                                  │
   │                  │                   │                │               ▼
   ▼                  ▼                   ▼                ▼          ┌──────────┐
┌────────┐      ┌──────────┐       ┌──────────┐    ┌───────────┐    │ Review   │
│Impact  │      │Fix in    │       │Code      │    │Fork-      │───▶│ staging  │
│map     │─────▶│fork      │──────▶│changes   │───▶│internal   │    │ PR then  │
│research│      │          │       │+ tests   │    │PR         │    │ create   │
└────────┘      └──────────┘       └──────────┘    └───────────┘    │upstream  │
                                                                     └──────────┘
```

## Creating a Fork Fix Spec

```bash
chant add "Fix issue #1234: Add locking to concurrent writes"
```

Edit the spec to reference all research phases:

```yaml
---
type: code
status: ready
depends_on:
  - 004-impact-map
target_files:
  - src/storage/store.rs
  - tests/storage/concurrent_test.rs
prompt: standard
informed_by:
  - .chant/research/issue-1234-root-cause.md
  - .chant/research/issue-1234-impact-map.md
---

# Phase 5: Fork Fix - Implement fix for Issue #1234

## Context

We have completed our research:
- Phase 3 identified the root cause: [brief summary]
- Phase 4 assessed the impact: [found N similar patterns]

Now we implement the fix in our fork before creating an upstream PR.

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

## Creating the Staging PR

After implementing the fix, create a fork-internal PR with proper structure:

```bash
# Push to your fork
git push origin fix/issue-1234

# Create PR against YOUR fork's main branch (not upstream)
# Note: The agent cannot create PRs without GitHub authentication setup.
# Human maintainers should create the staging PR manually:
gh pr create \
  --repo yourusername/project \
  --base main \
  --head yourusername:fix/issue-1234 \
  --title "Fix #1234: Data loss on concurrent writes" \
  --body "$(cat <<'EOF'
## User Impact

**What users see before this change:**
- Data loss during concurrent writes

**What users see after this change:**
- All concurrent writes succeed correctly

## Parent Context

**Related to:** #1234 (upstream issue)

This staging PR implements the fix for the data loss bug identified in research phases.

## Implementation

Research-backed fix using pessimistic locking. See .chant/research/ for full analysis.

## Testing

- Regression test passes
- All existing tests pass
- Added concurrency stress tests
EOF
)"
```

This staging PR:
- Runs CI in your fork
- Allows iteration without upstream noise
- Can be reviewed by the agent or other maintainers
- Serves as quality gate before upstream PR

## Connecting Research to Implementation

The implementation is informed by all research phases:

```yaml
depends_on:
  - 004-impact-map
informed_by:
  - .chant/research/issue-1234-root-cause.md
  - .chant/research/issue-1234-impact-map.md
```

The agent reads the research outputs, which provide:
- What the root cause is
- What files are affected
- What edge cases to handle
- What tests to add

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
depends_on:
  - 004-impact-map
target_files:
  - src/storage/store.rs
  - tests/storage/concurrent_test.rs
prompt: standard
informed_by:
  - .chant/research/issue-1234-root-cause.md
  - .chant/research/issue-1234-impact-map.md
commits:
  - ghi789b
completed_at: 2026-01-29T18:00:00Z
model: sonnet
---

# Phase 5: Fork Fix - Implement fix for Issue #1234

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
chant add "Re-research issue #1234: Pessimistic locking insufficient"
# Edit spec to set type: task
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

## Staging PR Review

The staging PR should be reviewed before creating the upstream PR:

1. **Automated checks:** CI must pass
2. **Agent review:** Optional agent-based review spec
3. **Manual review:** Maintainer reviews the staging PR
4. **Iteration:** Make changes until staging PR is approved

Once staging PR is approved, proceed to create the upstream PR (next phase).

## See Also

- [Impact Map Research](04-impact-map.md) — Previous step: identify all affected files
- [Upstream PR](06-upstream-pr.md) — Next step: human creates upstream PR
