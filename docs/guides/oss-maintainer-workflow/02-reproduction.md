# Reproducibility

Create a minimal failing test before diving into root cause analysis. Reproduction can be automatic (agent creates test) or assisted (agent provides instructions).

## Why Reproduce First?

Reproduction provides:

- **Confirmation** that the bug exists in your environment
- **Isolation** of the minimal conditions that trigger it
- **Baseline** to verify when the fix works
- **Documentation** for future regression testing

## Automatic vs Assisted Reproduction

**Automatic:** Agent creates a failing test that demonstrates the bug
- Best for: Unit-testable bugs, clear reproduction steps
- Output: Failing test file

**Assisted:** Agent provides instructions for manual reproduction
- Best for: UI bugs, system-level issues, environment-specific bugs
- Output: Step-by-step reproduction instructions

A reproduction spec produces either a failing test or reproduction instructions.

## Reproduction Workflow

```
Comprehension     Reproduction Spec      Failing Test       Research Input
   Output               │                    │                   │
      │                 ▼                    ▼                   ▼
      ▼           ┌───────────┐        ┌──────────┐       ┌───────────┐
┌───────────┐     │ Run       │        │ Minimal  │       │ informed  │
│ needs-    │     │ reproduce │        │ test     │       │ by: repro │
│ repro-    │────▶│ prompt    │───────▶│ fails    │──────▶│ spec      │
│ duction   │     └───────────┘        └──────────┘       └───────────┘
└───────────┘
```

## Creating a Reproduction Spec

```bash
chant add "Reproduce issue #1234: Data loss on concurrent writes"
```

Edit the spec:

```yaml
---
type: task
status: ready
prompt: reproduce
labels:
  - reproduction
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-001-abc.md  # Comprehension spec
  - https://github.com/yourproject/issues/1234
target_files:
  - tests/regression/issue_1234_test.rs
---

# Phase 2: Reproduction - Create failing test for Issue #1234

## Context

Based on phase 1 comprehension, we understand that:
- [Summary of key findings from comprehension phase]

## Issue Summary

User reports that saving a file while another write is in progress causes
data loss. Comprehension phase determined this needs reproduction before root cause research.

## Environment Details

From user report:
- OS: macOS 14.2
- Version: 0.5.2
- Concurrent writes via multiple CLI invocations

## Acceptance Criteria

- [ ] Minimal test case created that demonstrates the bug
- [ ] Test fails consistently (not flaky)
- [ ] Test is isolated (doesn't depend on external state)
- [ ] Environment details documented in test comments
- [ ] Test added to regression test suite
```

## The Reproduce Prompt

The `reproduce` prompt instructs the agent to create minimal reproductions:

```markdown
You are creating a minimal reproduction case for a reported issue.

Your goal is to:
1. Create the smallest possible test case that demonstrates the bug
2. Write a failing test (use project's test framework)
3. Document exact environment/version details
4. Validate that the reproduction matches the user's report

Output:
- Failing test file that proves the bug exists
- Reproduction steps documented in comments
- Environment details (OS, versions, config)
- Confirmation that bug is reproducible
```

## Reproduction Strategies

### Unit Test Reproduction

For isolated logic bugs:

```rust
#[test]
fn issue_1234_concurrent_write_loses_data() {
    // Reproduction for: https://github.com/yourproject/issues/1234
    // User: @reporter
    // Environment: macOS 14.2, version 0.5.2

    let store = Store::new_temp();

    // Write initial data
    store.write("key", "initial").unwrap();

    // Simulate concurrent writes
    let handle1 = store.write_async("key", "value1");
    let handle2 = store.write_async("key", "value2");

    handle1.join().unwrap();
    handle2.join().unwrap();

    // Bug: One write is lost instead of last-write-wins
    let result = store.read("key").unwrap();
    assert!(
        result == "value1" || result == "value2",
        "Expected one of the written values, got: {result}"
    );
}
```

### Integration Test Reproduction

For system-level bugs:

```rust
#[test]
fn issue_1234_concurrent_cli_invocations() {
    // Reproduction for: https://github.com/yourproject/issues/1234

    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Initialize file
    std::fs::write(&file_path, "initial").unwrap();

    // Run concurrent CLI commands
    let cmd1 = Command::new(env!("CARGO_BIN_EXE_yourproject"))
        .args(["write", file_path.to_str().unwrap(), "content1"])
        .spawn()
        .unwrap();

    let cmd2 = Command::new(env!("CARGO_BIN_EXE_yourproject"))
        .args(["write", file_path.to_str().unwrap(), "content2"])
        .spawn()
        .unwrap();

    cmd1.wait().unwrap();
    cmd2.wait().unwrap();

    // Verify data integrity
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(
        content == "content1" || content == "content2",
        "Data corrupted: {content}"
    );
}
```

### Stress Test Reproduction

For race conditions:

```rust
#[test]
fn issue_1234_race_condition_stress() {
    // Run multiple times to catch intermittent race
    for iteration in 0..100 {
        let store = Store::new_temp();

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let store = store.clone();
                std::thread::spawn(move || {
                    store.write("key", &format!("value{i}"))
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap().unwrap();
        }

        let result = store.read("key").unwrap();
        assert!(
            result.starts_with("value"),
            "Iteration {iteration}: Data corrupted: {result}"
        );
    }
}
```

## Incremental Research Documentation

**IMPORTANT:** Update your research document incrementally as findings emerge, not at the end.

As you work through reproduction:
- Document each finding in the research doc immediately
- Update the spec body with observations as you discover them
- Keep a running log of what you tried and what you learned

This creates an accurate paper trail and helps the next stage build on documented findings rather than relying on context window memory.

**Example incremental updates:**
```markdown
## Reproduction Log

**Initial attempt (14:30):** Tried simple concurrent write test. Reproduced consistently.

**Observation (14:45):** Issue only occurs with >2 concurrent writers. Single writer works fine.

**Discovery (15:00):** Added timing instrumentation. Gap between read and write is critical window.

**Confirmed (15:20):** Issue is timing-dependent race condition, not logic bug.
```

## Reproduction Output

A successful reproduction spec produces:

1. **Failing test file** at the `target_files` location
2. **Test comments** documenting the issue and environment
3. **Consistent failures** (test fails reliably, not flaky)

Example test output:

```
running 1 test
test regression::issue_1234_concurrent_write_loses_data ... FAILED

failures:

---- regression::issue_1234_concurrent_write_loses_data stdout ----
thread 'regression::issue_1234_concurrent_write_loses_data' panicked at
tests/regression/issue_1234_test.rs:23:5:
Expected one of the written values, got: valu
```

## When Reproduction Fails

If you can't reproduce the bug:

### Environment Mismatch

```markdown
## Reproduction Attempt

**Result:** Cannot reproduce

**Tested environments:**
- macOS 14.2, version 0.5.2 (matches user report)
- Linux Ubuntu 22.04, version 0.5.2
- macOS 14.2, version 0.5.1

**Observations:**
- All tests pass
- Concurrent writes behave correctly

**Next steps:**
- Ask user for more specific reproduction steps
- Request debug logs from user's environment
- Check if user has custom configuration
```

### Flaky Reproduction

```markdown
## Reproduction Attempt

**Result:** Intermittent

**Observations:**
- Bug reproduces ~10% of the time
- More likely with higher concurrency (>5 threads)
- Never seen on single-core machines

**Test approach:**
- Stress test with 100 iterations
- Fails ~8 times per run

**Recommendation:**
- Proceed to research with stress test
- Note intermittent nature for root cause analysis
```

### Cannot Reproduce, User Error

```markdown
## Reproduction Attempt

**Result:** User configuration issue

**Observations:**
- User had outdated config file
- Documented behavior works correctly
- Issue resolved by config migration

**Recommendation:**
- Close issue with explanation
- Improve config migration documentation
- Consider adding config validation warning
```

## Spec Completion

When reproduction succeeds:

```yaml
---
type: task
status: completed
prompt: reproduce
labels:
  - reproduction
  - issue-1234
informed_by:
  - .chant/specs/2026-01-29-001-abc.md
target_files:
  - tests/regression/issue_1234_test.rs
model: claude-sonnet-4-20250514
completed_at: 2026-01-29T14:30:00Z
---
```

The completed spec becomes `informed_by` input for the research phase.

## Reproduction Test Conventions

Establish conventions for regression tests:

```rust
// File: tests/regression/mod.rs

// Naming: issue_{number}_{brief_description}
// Location: tests/regression/issue_{number}_test.rs
// Comment block: Issue URL, reporter, environment, date

/// Regression tests for reported issues.
///
/// Each test should:
/// 1. Reference the issue number in the name
/// 2. Include issue URL in comments
/// 3. Document reproduction environment
/// 4. Be self-contained (no external dependencies)
mod issue_1234_concurrent_write;
mod issue_1235_unicode_filenames;
```

## Assisted Reproduction Example

When automatic reproduction isn't possible:

```markdown
## Assisted Reproduction Instructions

**Environment:** macOS 14.2, requires physical display

**Steps:**
1. Open the application
2. Navigate to Settings > Display
3. Change resolution to 1920x1080
4. Click "Apply" twice rapidly

**Expected:** Settings should save
**Actual:** Second click causes UI freeze

**Additional context:**
- Only occurs on macOS with Retina displays
- Requires physical interaction (cannot be automated)
- Happens ~80% of the time

**For developer:**
- Check event queue handling in `src/ui/settings.rs:234`
- Suspect race condition in resolution change handler
```

## See Also

- [Comprehension Research](01-comprehension.md) — Previous step: understand the issue
- [Root Cause Research](03-root-cause.md) — Next step: investigate why the bug exists
