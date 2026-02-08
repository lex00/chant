# Lifecycle Walkthrough: Building an Export Command

This guide walks through the complete spec lifecycle using a realistic scenario. Each of the ten lifecycle phases appears naturally as the feature progresses from idea to long-term maintenance. Command examples and output are illustrative — your exact output will differ.

## Scenario

You're maintaining `datalog`, a Python CLI tool for analyzing logs. A user requests an export feature to save query results as CSV. You'll use chant to manage this work.

## Phase 1: Create

You start by creating a spec:

```bash
$ chant add "Add export command to datalog CLI"
Created spec: 2026-02-08-001-xyz
```

You open the spec and write out the full requirements — CSV and JSON formats, streaming support, compression, custom field selection. By the time you're done, the spec has 12 acceptance criteria and 8 target files. Chant lints it automatically:

```
Lint diagnostics:
  [WARNING] complexity: Spec has 12 acceptance criteria (threshold: 10)
    → Consider splitting into smaller, focused specs
  [WARNING] complexity: Spec has 8 target files (threshold: 5)
    → Consider splitting into smaller, focused specs
  [WARNING] complexity: Spec description is 450 words (threshold: 200)
    → Consider splitting into smaller, focused specs
```

The linter is telling you: this spec is trying to do too much. You edit it down to basic CSV export only:

```bash
$ chant edit 001
```

After saving, chant re-lints:

```
Lint diagnostics:
  [WARNING] complexity: Spec has 6 acceptance criteria (threshold: 5)
    → Borderline complexity
```

Better, but still borderline. Time to split.

## Phase 2: Split

`chant split` analyzes the spec and proposes a breakdown:

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

The original spec becomes a **driver** that coordinates three **member** specs. Each member is focused on one concern:

```bash
$ chant list --type driver

ID          Type    Status   Title
001-xyz     driver  pending  Add export command to datalog CLI
```

## Phase 3: Dependencies

The members have a natural ordering. The command skeleton needs the CSV handler to exist first, and integration tests need both:

```bash
$ chant list --ready

ID          Type  Status  Title
001.1-xyz   code  ready   Add CSV export format handler
```

Only `001.1` is ready because `001.2` depends on it, and `001.3` depends on both. The dependency graph makes this explicit:

```bash
$ chant dag

001-xyz (driver)
├── 001.1-xyz  Add CSV export format handler
├── 001.2-xyz  Implement export command skeleton
│   depends_on: [001.1-xyz]
└── 001.3-xyz  Add integration tests for export
    depends_on: [001.1-xyz, 001.2-xyz]
```

## Phase 4: Execute

Chain mode processes each spec in dependency order, spawning an agent in an isolated worktree for each:

```bash
$ chant work --chain

[1/3] Working 001.1-xyz: Add CSV export format handler
→ Agent working in worktree /tmp/chant-001.1-xyz
...
✓ Completed in 2m 15s
→ Merged to main, worktree cleaned up

[2/3] Working 001.2-xyz: Implement export command skeleton
→ Agent working in worktree /tmp/chant-001.2-xyz
...
✓ Completed in 1m 45s
→ Merged to main, worktree cleaned up

[3/3] Working 001.3-xyz: Add integration tests for export
→ Agent working in worktree /tmp/chant-001.3-xyz
...
```

Each agent works in its own worktree so there are no conflicts. When an agent finishes, its changes are merged to main and the worktree is cleaned up before the next spec starts.

## Phase 5: Failure

The third spec fails — the agent's tests discover an edge case with empty datasets:

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

Chain mode stops on failure rather than continuing. You inspect the damage:

```bash
$ chant show 001.3

ID:     2026-02-08-001-xyz.3
Status: failed
Title:  Add integration tests for export
```

The log shows what went wrong:

```bash
$ chant log 001.3

[2026-02-08 14:32:00] Running integration tests...
[2026-02-08 14:32:15] ✗ Test failed: export_empty_dataset
[2026-02-08 14:32:15] Error: IndexError: list index out of range
[2026-02-08 14:32:15]   at src/export.py:18, in export_csv
```

The CSV handler assumed at least one row of data.

## Phase 6: Recover

You fix the edge case, then reset and retry:

```bash
# Fix the bug
$ vim src/export.py

# Reset the spec to pending and re-execute
$ chant reset 001.3
Spec 001.3-xyz reset to pending

$ chant work 001.3
Working 001.3-xyz: Add integration tests for export
...
✓ Completed in 1m 10s (attempt 2)
```

`chant reset` transitions from `failed` back to `pending` and increments the retry counter. The agent gets a fresh worktree and tries again.

## Phase 7: Complete

All members are now complete, and the driver auto-completed with them:

```bash
$ chant list --status completed

ID          Type    Status     Title
001-xyz     driver  completed  Add export command to datalog CLI
001.1-xyz   code    completed  Add CSV export format handler
001.2-xyz   code    completed  Implement export command skeleton
001.3-xyz   code    completed  Add integration tests for export
```

No manual cleanup needed — worktrees are gone, branches merged and deleted, the feature is on main:

```bash
$ chant worktree status
No active worktrees

$ git branch
* main
```

## Phase 8: Verify

Before shipping, you verify that acceptance criteria still hold against the actual codebase:

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

Verification updates the spec frontmatter with a timestamp and status:

```yaml
---
status: completed
last_verified: 2026-02-08T15:00:00Z
verification_status: passed
---
```

## Phase 9: Drift

Three weeks later, the data model changes — a new `severity` field is added to log entries. Your CI pipeline runs nightly verification:

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

The spec frontmatter records what drifted:

```yaml
---
status: completed
last_verified: 2026-03-01T03:00:00Z
verification_status: partial
verification_failures:
  - "Tests verify all fields exported (new field 'severity' missing)"
---
```

The spec is still completed — the original work was correct. But reality has moved on, and verification caught the gap.

## Phase 10: React

You create a follow-up spec to address the drift, linking it to the original for traceability:

```bash
$ chant add "Add severity field to CSV export"
Created spec: 2026-03-01-004-abc

$ chant edit 004
# Add to frontmatter:
# informed_by: [2026-02-08-001-xyz]
```

Execute and verify:

```bash
$ chant work 004
Working 004-abc: Add severity field to CSV export
...
✓ Completed in 45s

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

The cycle is complete. The original spec verifies clean again, and the fix is tracked as a separate spec with a clear link to what prompted it.

## Summary

| Concept | What it does |
|---------|-------------|
| **Auto-lint** | Catches complexity early at creation time |
| **Split** | Breaks large specs into focused members with a driver |
| **Dependencies** | `depends_on` ensures correct execution order |
| **Chain mode** | `--chain` processes ready specs sequentially |
| **Worktree isolation** | Each agent works in its own git worktree |
| **Retry tracking** | `reset` increments attempt counter |
| **Driver auto-completion** | Driver completes when all members finish |
| **Verification** | Re-checks criteria against current codebase |
| **Drift detection** | Catches when code diverges from spec intent |
| **Follow-up specs** | `informed_by` links new work to what prompted it |

## Reference Implementation

A **[reference implementation](workflows/lifecycle-walkthrough/artifacts/)** accompanies this guide with concrete examples of each phase:

- **Source code** — A minimal Python `datalog` CLI tool (`src/datalog.py`, `src/query.py`, `src/export.py`) with tests
- **Spec files** — Pre-built specs showing what each phase produces:
  - `spec-001-initial.md` — The overly complex initial spec (Phase 1)
  - `spec-001-focused.md` — After editing down (Phase 1)
  - `spec-001-driver.md` — After splitting into driver (Phase 2)
  - `spec-001.1-csv-handler.md` — CSV handler member (Phase 2)
  - `spec-001.2-command-skeleton.md` — Command skeleton member (Phase 2)
  - `spec-001.3-integration-tests.md` — Integration tests member (Phase 2)
  - `spec-004-severity-field.md` — Drift follow-up spec (Phase 10)
- **Test script** — `test.sh` validates the artifacts (source files, spec frontmatter, Python syntax, chant operations)

See the [artifacts README](workflows/lifecycle-walkthrough/artifacts/README.md) for details.

## Further Reading

- [Lifecycle](../concepts/lifecycle.md) — State machine and transitions
- [Dependencies](../concepts/deps.md) — Dependency resolution and blocking
- [CLI Reference](../reference/cli.md) — Full command documentation
- [MCP Operations](../reference/mcp.md) — Using chant via MCP tools
