# Lifecycle Walkthrough: Building an Export Command

This guide walks through the complete spec lifecycle by building a realistic feature: adding an `export` command to an existing CLI tool. You'll see every lifecycle phase naturally unfold, from initial creation through drift detection weeks later.

## Scenario

You're maintaining `datalog`, a CLI tool for analyzing logs. A user requests an export feature to save query results as CSV or JSON. You'll use chant to manage this work, encountering each lifecycle phase along the way.

## Phase 1: Create

Start by creating a spec for the export feature:

```bash
$ chant add "Add export command to datalog CLI"
Created spec: 2026-02-08-001-xyz
```

Chant automatically lints the new spec and provides feedback:

```
Lint diagnostics:
  [WARNING] complexity: Spec has 12 acceptance criteria (threshold: 10)
    → Consider splitting into smaller, focused specs
  [WARNING] complexity: Spec has 8 target files (threshold: 5)
    → Consider splitting into smaller, focused specs
  [WARNING] complexity: Spec description is 450 words (threshold: 200)
    → Consider splitting into smaller, focused specs
```

You realize the spec is too complex. Edit it to be more focused:

```bash
$ chant edit 001
```

Reduce the scope to basic CSV export only. After saving, chant re-lints automatically:

```
Lint diagnostics:
  [WARNING] complexity: Spec has 6 acceptance criteria (threshold: 5)
    → Borderline complexity
```

Better, but still a bit much. Time to split.

## Phase 2: Split

Use `chant split` to break down the complex spec:

```bash
$ chant split 001
Analyzing spec 001...

Recommended split into 3 member specs:
  001.1 - Add CSV export format handler
  001.2 - Implement export command skeleton
  001.3 - Add integration tests for export

Split into members? [y/N] y

Created:
  2026-02-08-001-xyz.1.md
  2026-02-08-001-xyz.2.md
  2026-02-08-001-xyz.3.md

Updated 001 to driver type with dependencies
```

The driver spec (001) now coordinates the three members. Check the structure:

```bash
$ chant list --type driver

ID          Type    Status   Title
001-xyz     driver  pending  Add export command to datalog CLI
```

## Phase 3: Dependencies

The members have dependencies on each other. Check which specs are ready:

```bash
$ chant ready

ID          Type  Status  Title
001.1-xyz   code  ready   Add CSV export format handler
```

Only `001.1` is ready because `001.2` depends on `001.1`, and `001.3` depends on both. View the dependency graph:

```bash
$ chant dag

001-xyz (driver)
├── 001.1-xyz  Add CSV export format handler
├── 001.2-xyz  Implement export command skeleton
│   depends_on: [001.1-xyz]
└── 001.3-xyz  Add integration tests for export
    depends_on: [001.1-xyz, 001.2-xyz]
```

The dependency chain ensures work happens in the right order.

## Phase 4: Execute

Start working through the specs:

```bash
$ chant work --chain

[1/3] Working 001.1-xyz: Add CSV export format handler
→ Starting watch (auto-started)
→ Agent working in worktree /tmp/chant-001.1-xyz
...
✓ Completed in 2m 15s
→ Merged to main, branch deleted, worktree cleaned up

[2/3] Working 001.2-xyz: Implement export command skeleton
→ Agent working in worktree /tmp/chant-001.2-xyz
...
✓ Completed in 1m 45s
→ Merged to main, branch deleted, worktree cleaned up

[3/3] Working 001.3-xyz: Add integration tests for export
→ Agent working in worktree /tmp/chant-001.3-xyz
...
```

**Under the hood:**
- `chant work --chain` auto-starts watch if not running
- Watch monitors worktrees for agent status changes (`.chant-status.json`)
- Each agent works in isolation in its own worktree
- When agent writes `status: done`, watch merges, finalizes, and cleans up
- Next ready spec automatically starts

## Phase 5: Failure

The third spec fails—tests discover an edge case with empty datasets:

```
[3/3] Working 001.3-xyz: Add integration tests for export
✗ Failed: Test failed: export panics on empty query result

════════════════════════════════════════════════════════════
Chain execution stopped:
  ✓ 2 specs completed
  ✗ 1 spec failed: 001.3-xyz
  Total time: 5m 30s
════════════════════════════════════════════════════════════
```

Check the spec status:

```bash
$ chant show 001.3

ID:     2026-02-08-001-xyz.3
Status: failed
Title:  Add integration tests for export
...
```

View the agent's log to understand the failure:

```bash
$ chant log 001.3

[2026-02-08 14:32:00] Running integration tests...
[2026-02-08 14:32:15] ✗ Test failed: export_empty_dataset
[2026-02-08 14:32:15] Error: panic: runtime error: slice bounds out of range
[2026-02-08 14:32:15]   at src/export.rs:42
```

The CSV export handler doesn't handle empty result sets.

## Phase 6: Recover

Fix the edge case manually, then retry the spec:

```bash
# Fix the bug in src/export.rs
$ vim src/export.rs

# Reset the spec to pending and re-execute
$ chant reset 001.3
Spec 001.3-xyz reset to pending

$ chant work 001.3
Working 001.3-xyz: Add integration tests for export
...
✓ Completed in 1m 10s (attempt 2)
```

**Under the hood:**
- `chant reset` transitions from `failed` → `pending`
- Retry counter increments (attempt 2)
- Watch handles the retry just like the initial attempt

All members are now complete. Check the driver:

```bash
$ chant show 001

ID:     2026-02-08-001-xyz
Type:   driver
Status: completed (auto-completed when all members finished)
```

## Phase 7: Complete

The driver auto-completes when all members finish. View the final state:

```bash
$ chant list --status completed

ID          Type    Status     Title
001-xyz     driver  completed  Add export command to datalog CLI
001.1-xyz   code    completed  Add CSV export format handler
001.2-xyz   code    completed  Implement export command skeleton
001.3-xyz   code    completed  Add integration tests for export
```

All worktrees have been cleaned up:

```bash
$ chant worktree status
No active worktrees
```

All branches merged and deleted:

```bash
$ git branch
* main
```

**Under the hood:**
- Watch merged each branch to main after agent completion
- Worktrees removed immediately after merge
- Branches deleted after successful merge
- Driver status auto-updated to `completed` when last member finished

## Phase 8: Verify

Before deploying, verify acceptance criteria still hold:

```bash
$ chant verify 001

Verifying spec 001-xyz: Add export command to datalog CLI

Checking acceptance criteria...
  ✓ CSV export format handler exists
  ✓ Export command available in CLI
  ✓ Integration tests pass
  ✓ Edge cases handled (empty datasets, large results)

Spec 001-xyz: VERIFIED
```

Verification updates the spec frontmatter:

```yaml
---
status: completed
last_verified: 2026-02-08T15:00:00Z
verification_status: passed
---
```

## Phase 9: Drift

Three weeks later, the data model changes—a new field is added to log entries. Your CI pipeline runs nightly verification and detects drift:

```bash
$ chant verify --all

Verifying 45 completed specs...

✓ 001.1-xyz: PASSED
✓ 001.2-xyz: PASSED
⚠ 001.3-xyz: PARTIAL (1 criterion skipped)
  Skipped: "Tests verify all fields exported"
  Reason: New field 'severity' not included in CSV export

Verification Summary:
  Passed: 43
  Partial: 2
  Failed: 0
```

The spec frontmatter is updated:

```yaml
---
status: completed
last_verified: 2026-03-01T03:00:00Z
verification_status: partial
verification_failures:
  - "Tests verify all fields exported (new field 'severity' missing)"
---
```

**Under the hood:**
- `chant verify` re-checks acceptance criteria against current codebase
- Agent reads spec and validates each criterion
- Detects that new `severity` field isn't in CSV export
- Updates verification status in frontmatter

## Phase 10: React

Create a follow-up spec to handle the drift:

```bash
$ chant add "Add severity field to CSV export"
Created spec: 2026-03-01-004-abc

# Link to original spec for context
$ chant edit 004
# Add to frontmatter:
# informed_by: [2026-02-08-001-xyz]
```

Execute the fix:

```bash
$ chant work 004
Working 004-abc: Add severity field to CSV export
...
✓ Completed in 45s
```

Verify the original spec again:

```bash
$ chant verify 001.3

Verifying spec 001.3-xyz: Add integration tests for export

Checking acceptance criteria...
  ✓ CSV export format handler exists
  ✓ Export command available in CLI
  ✓ Integration tests pass
  ✓ Edge cases handled
  ✓ Tests verify all fields exported (severity now included)

Spec 001.3-xyz: VERIFIED
```

The cycle is complete. The original spec is verified clean, and the fix is tracked separately.

## Summary

You've experienced the full lifecycle:

1. **Create** — Spec created, auto-linted with feedback
2. **Split** — Complex spec split into driver + members
3. **Dependencies** — Members ordered via dependency chain
4. **Execute** — Chain execution with worktree isolation
5. **Failure** — Tests revealed edge case
6. **Recover** — Manual fix + retry with attempt tracking
7. **Complete** — All members done, driver auto-completes, cleanup automatic
8. **Verify** — Acceptance criteria re-checked before deploy
9. **Drift** — Weeks later, verification detects model change
10. **React** — Follow-up spec created to address drift

## Key Concepts Demonstrated

- **Auto-lint** catches complexity early
- **Split** breaks large work into manageable pieces
- **Dependencies** (`depends_on`) ensure correct execution order
- **Chain mode** (`--chain`) processes specs sequentially
- **Watch** auto-starts and handles lifecycle (merge, finalize, cleanup)
- **Worktree isolation** prevents conflicts during execution
- **Retry tracking** increments attempt counter on `reset`
- **Driver auto-completion** when all members finish
- **Verification** re-checks criteria against current codebase
- **Drift detection** catches when reality diverges from intent
- **Follow-up specs** handle changes over time

## Further Reading

- [Lifecycle](../concepts/lifecycle.md) — State machine and transitions
- [Dependencies](../concepts/deps.md) — Dependency resolution and blocking
- [CLI Reference](../reference/cli.md) — Full command documentation
- [MCP Operations](../reference/mcp.md) — Using chant via MCP tools
