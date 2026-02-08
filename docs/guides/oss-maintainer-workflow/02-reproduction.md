# Phase 2: Reproduction

With the comprehension document identifying the affected code areas, you now need to prove the bug exists in your environment. A failing test serves as both confirmation and as the success criterion for the eventual fix.

## Creating the Reproduction Spec

```bash
$ chant add "Reproduce issue #1234: data loss on concurrent writes"
Created spec: 2026-02-08-004-m2p
```

You edit the spec as a task that references the comprehension output:

```yaml
---
type: task
prompt: reproduce
labels: [reproduction, issue-1234]
informed_by:
  - .chant/specs/2026-02-08-001-r4x.md
  - .chant/research/issue-1234-comprehension.md
target_files:
  - tests/regression/issue_1234_test.rs
---
```

The `informed_by` chain is important here. The agent reads the comprehension document to know which files and components to focus on, rather than starting from scratch.

```bash
$ chant work 004
Working 004-m2p: Reproduce issue #1234: data loss on concurrent writes
> Agent working in worktree /tmp/chant-004-m2p
...
Completed in 2m 10s
```

## What the Agent Produces

The agent writes a minimal failing test:

```rust
#[test]
fn issue_1234_concurrent_write_loses_data() {
    // Reproduction for: https://github.com/yourproject/kvstore/issues/1234
    // Environment: macOS 14.2, version 0.5.2
    let store = Store::new_temp();
    store.write("key", "initial").unwrap();

    let handle1 = store.write_async("key", "value1");
    let handle2 = store.write_async("key", "value2");

    handle1.join().unwrap();
    handle2.join().unwrap();

    let result = store.read("key").unwrap();
    assert!(
        result == "value1" || result == "value2",
        "Expected one of the written values, got: {result}"
    );
}
```

Running it confirms the bug:

```
running 1 test
test regression::issue_1234_concurrent_write_loses_data ... FAILED

failures:
    Expected one of the written values, got: valu
```

The truncated value `valu` is telling. This isn't just a last-write-wins ordering issue -- data is being partially overwritten. The write operation is not atomic.

## Automatic vs Assisted Reproduction

The approach above is automatic reproduction: the agent writes and runs a test. For bugs that can't be captured in an automated test -- UI issues, environment-specific problems, hardware-dependent behavior -- the agent instead produces reproduction instructions that a human follows.

## When Reproduction Fails

Three outcomes when you can't reproduce:

**Environment mismatch.** The bug depends on a specific OS, filesystem, or configuration you don't have. Ask the user for more details or debug logs.

**Flaky reproduction.** The bug appears intermittently. For a race condition like this one, a stress test with multiple iterations often catches it:

```rust
#[test]
fn issue_1234_race_condition_stress() {
    for _ in 0..100 {
        let store = Store::new_temp();
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let s = store.clone();
                std::thread::spawn(move || s.write("key", &format!("v{i}")))
            })
            .collect();
        for h in handles { h.join().unwrap().unwrap(); }
        let result = store.read("key").unwrap();
        assert!(result.starts_with("v"), "Data corrupted: {result}");
    }
}
```

**User error.** The reported behavior is caused by misconfiguration or outdated software. Document the finding, suggest the fix, and consider improving error messages or documentation.

## Documenting Incrementally

As the agent works through reproduction, it should update its findings as they emerge rather than writing everything at the end. This creates an accurate paper trail and prevents loss of observations if the agent's context is interrupted. The reproduction spec captures not just the final test, but what was tried along the way.

**Next:** [Root Cause](03-root-cause.md)
